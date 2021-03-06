#![feature(box_syntax, box_patterns)]

extern crate image;
#[macro_use]
extern crate log;

#[cfg(not(test))]
#[macro_use]
extern crate glium;

use std::cmp;
use std::fmt;
use std::fs::File;
use std::marker::PhantomData;
use std::mem;
use std::process::Command;

use presses::{Paper, Press};

mod presses;
#[cfg(not(test))]
mod scene;

/// How much space a subtree takes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Bound(u32, u32);

/// Minimum envelope around a token.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Fit(u32, u32);

/// Subtree position.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Pos(i32, i32);


fn build_math() -> Body {
    use Expr::*;

    // IC
    let mut body = Body { stmts: vec![] };
    // 'print'
    body.stmts.push(Stmt::Print(Hole));
    // '4'
    match body.stmts[0] {
        Stmt::Print(ref mut hole) => *hole = Int(4), // `print(4)`
        _ => panic!(),
    }
    // '+'
    match body.stmts[0] {
        Stmt::Print(ref mut expr) => {
            let mut hand = Plus(box Hole, box Hole);
            mem::swap(expr, &mut hand); // `print(_ + _)`
            match *expr {
                Plus(box ref mut left, _) => {
                    mem::swap(left, &mut hand) // `print(4 + _)`
                }
                _ => panic!()
            }
        }
        _ => panic!()
    }
    // '2 -'
    let v = Var;
    match body.stmts[0] {
        Stmt::Print(Plus(_, box ref mut right)) => {
            let bind = Bind(Ref::new(0)); // need to bind `v` somehow

            *right = Minus(box Int(2), box bind); // `print(4 + (2 - x))`
        }
        _ => panic!()
    }
    // prepend 'let x = 1'
    body.stmts.insert(0, Stmt::Let(v, Int(1)));

    body
}

fn draw_math<I: Paper>(math: &Body, paper: &mut I) {
    let (branches, tokens) = grow_tree(math);
    let tree = Tree::new(&branches[..]);
    draw_tree(&tree, &tokens[..], paper);
}

fn draw_tree<I: Paper>(tree: &Tree, tokens: &[String], paper: &mut I) {
    let ref press = presses::FreeTypePress::new().unwrap();

    const N: usize = 10;
    assert_eq!(tree.len(), N);
    assert_eq!(tokens.len(), N);

    let mut c_fit = [Fit(0, 0); N];
    measure_fits(tokens, press, &mut c_fit);

    let mut c_bound = [Bound(0, 0); N];
    compute_bounds(&tree, &c_fit, &mut c_bound);

    let mut c_pos = [Pos(0, 0); N];
    compute_positions(&tree, &c_fit, &c_bound, &mut c_pos);

    for ix in 0..N {
        let Pos(x, y) = c_pos[ix];
        // blit_str should take Write or something to avoid temporary
        let ref s = tokens[ix];
        press.blit_str(s, (x, y), paper).unwrap();
    }
}

/// Determines how many children a tree node has.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Branch(u32);

/// Represents a tree hierarchy.
pub struct Tree<'a> {
    pub branches: &'a [Branch],
}

impl<'a> Tree<'a> {
    pub fn new(branches: &'a [Branch]) -> Self {
        assert!(branches.len() > 0, "tree must have root");
        Tree {
            branches: branches,
        }
    }

    pub fn len(&self) -> usize {
        self.branches.len()
    }

    /// Depth-first bottom-up pass over the tree.
    ///
    /// Values of type R originate in leaves, propagating to the root.
    pub fn flow_up<F, R>(&self, mut f: F) -> R
        where F: FnMut(usize, &[R]) -> R {

        let (ix, r) = self.flow_up_subtree(&mut f, 0);
        // check that we actually saw the whole tree
        assert_eq!(ix, self.len());
        r
    }

    /// So is there a carrot to use this?
    /// Safe, easy parallelism? Or is this just for show?
    fn flow_up_subtree<F, R>(&self, f: &mut F, root_ix: usize) -> (usize, R)
        where F: FnMut(usize, &[R]) -> R {

        // descend first to leaves
        let Branch(n) = self.branches[root_ix];
        let mut child_ix = root_ix + 1;

        let r = if n > 0 {
            // gaaah, allocation.
            let mut rets = Vec::with_capacity(n as usize);
            for _ in 0..n {
                // can these three be combined into one?
                let (new_ix, r) = self.flow_up_subtree(f, child_ix);
                child_ix = new_ix;
                rets.push(r);
            }
            f(root_ix, &rets)
        }
        else {
            f(root_ix, &[])
        };

        (child_ix, r)
    }
}

fn measure_fits<P: Press>(tokens: &[String], press: &P, fits: &mut [Fit]) {
    for (ix, ref text) in tokens.iter().enumerate() {
        let (w, h) = press.measure_str(text).unwrap();
        fits[ix] = Fit(w, h);
    }
}

fn compute_bounds(tree: &Tree, fits: &[Fit], bounds: &mut [Bound]) {
    // measure everything, starting bottom-up
    // currently nothing fancy like margins or padding
    let _total_bounds = tree.flow_up(|ix, child_bounds| -> Bound {

        let Fit(w, h) = fits[ix];

        // boundary is sum of widths and max of heights
        let bound = child_bounds.iter().fold(Bound(w, h), |total, child: &Bound| {
            let sum_w = child.0 + total.0;
            let max_h = cmp::max(child.1, total.1);
            Bound(sum_w, max_h)
        });
        bounds[ix] = bound;
        bound
    });
}

fn compute_positions(tree: &Tree, fits: &[Fit], bounds: &[Bound], coords: &mut [Pos]) {
    assert!(bounds.len() >= tree.len(), "Not enough Bounds allocated");
    assert!(coords.len() >= tree.len(), "Not enough Pos' allocated");

    let mut stack = vec![(Pos(0, 0), 1)];
    for ix in 0..tree.len() {
        // this is pretty awkward. maybe ditch last_mut?
        let (pop, push) = {
            let (ref mut cursor, ref mut n_siblings) =
                    *stack.last_mut().expect("unexpected end of tree");

            coords[ix] = *cursor;

            // if we have children, push the cursor and sibling count on the stack
            let Branch(n) = tree.branches[ix];
            let push = if n > 0 {
                let mut child_pos = *cursor;
                child_pos.0 += fits[ix].0 as i32;
                child_pos.1 += 5;
                Some((child_pos, n))
            }
            else {
                None
            };

            // if we're done at this level, pop back out to our old cursor
            let bound = bounds[ix];
            cursor.0 += bound.0 as i32;
            assert!(*n_siblings > 0);
            *n_siblings -= 1;
            let pop = *n_siblings == 0;

            (pop, push)
        };
        if pop {
            stack.pop().unwrap();
        }
        if let Some(state) = push {
            stack.push(state);
        }
    }
    assert_eq!(stack.len(), 0);
}

/// The product of growing a Tree.
pub trait Wood : fmt::Display {
    fn branching_factor(&self) -> usize;
}

/// Anything that can germinate a Tree of Ts.
pub trait Seed {
    fn germinate<F: FnMut(&Wood)>(&self, shoot: &mut F);
}

fn grow_tree<S: Seed>(seed: &S) -> (Vec<Branch>, Vec<String>) {
    let mut branches = vec![];
    let mut tokens = vec![];
    seed.germinate(&mut |token| {
        let branch = Branch(token.branching_factor() as u32);
        branches.push(branch);
        let token = format!("{}", token);
        tokens.push(token);
    });
    (branches, tokens)
}

///////////////// REFS /////////////////

// The naked <T> may be inappropriate.
#[derive(Debug, Eq, PartialEq)]
pub struct Ref<T> {
    pub id: RefId,
    _phantom: PhantomData<T>,
}

// '?Sized'?
impl<T> Copy for Ref<T> {}
impl<T> Clone for Ref<T> { fn clone(&self) -> Self { *self } }

impl<T> Ref<T> {
    fn new(id: RefId) -> Self {
        Ref {id: id, _phantom: PhantomData}
    }
}

pub type RefId = usize;

///////////////// REFERENT /////////////

pub struct Var;

pub struct VarBark;

impl Wood for VarBark {
    fn branching_factor(&self) -> usize { 0 }
}

impl fmt::Display for VarBark {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "var")
    }
}

impl Seed for Var {
    fn germinate<F: FnMut(&Wood)>(&self, shoot: &mut F) {
        shoot(&VarBark);
    }
}

///////////////// EXPR /////////////////

pub enum Expr {
    Hole,
    Bind(Ref<Var>),
    Int(i32),
    Minus(Box<Expr>, Box<Expr>),
    Plus(Box<Expr>, Box<Expr>),
}

pub enum ExprBark {
    Hole,
    Bind(Ref<Var>),
    Int(i32),
    Minus,
    Plus,
}

impl Wood for ExprBark {
    fn branching_factor(&self) -> usize {
        use ExprBark::*;
        match *self {
            Hole => 0,
            Bind(_) => 0,
            Int(_) => 0,
            Minus => 2,
            Plus => 2,
        }
    }
}

impl fmt::Display for ExprBark {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ExprBark::*;
        match *self {
            Hole => write!(f, "_"),
            Bind(_) => write!(f, "x"),
            Int(i) => write!(f, "{}", i),
            Minus => write!(f, "-"),
            Plus => write!(f, "+"),
        }
    }
}

// ought to be auto-derived
impl Seed for Expr {
    fn germinate<F: FnMut(&Wood)>(&self, shoot: &mut F) {
        use Expr::*;
        match *self {
            Hole => shoot(&ExprBark::Hole),
            Bind(ref r) => shoot(&ExprBark::Bind(*r)),
            Int(i) => shoot(&ExprBark::Int(i)),
            Minus(ref left, ref right) => {
                shoot(&ExprBark::Minus);
                left.germinate(shoot);
                right.germinate(shoot);
            }
            Plus(ref left, ref right) => {
                shoot(&ExprBark::Plus);
                left.germinate(shoot);
                right.germinate(shoot);
            }
        }
    }
}

////////////////// STMT ////////////////

pub enum Stmt {
    Let(Var, Expr),
    Print(Expr),
}

pub enum StmtBark {
    Let,
    Print,
}

impl fmt::Display for StmtBark {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use StmtBark::*;
        match *self {
            Let => write!(f, "let"),
            Print => write!(f, "print"),
        }
    }
}

impl Wood for StmtBark {
    fn branching_factor(&self) -> usize {
        use StmtBark::*;
        match *self {
            Let => 2,
            Print => 1,
        }
    }
}

// ought to be auto-derived
impl Seed for Stmt {
    fn germinate<F: FnMut(&Wood)>(&self, shoot: &mut F) {
        use Stmt::*;
        match *self {
            Let(ref var, ref expr) => {
                shoot(&StmtBark::Let);
                var.germinate(shoot);
                expr.germinate(shoot);
            }
            Print(ref expr) => {
                shoot(&StmtBark::Print);
                expr.germinate(shoot);
            }
        }
    }
}

pub struct Body {
    pub stmts: Vec<Stmt>,
}

pub struct BodyBark(usize);

impl fmt::Display for BodyBark {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "body: ")
    }
}

impl Wood for BodyBark {
    fn branching_factor(&self) -> usize { self.0 }
}

// ought to be auto-derived
impl Seed for Body {
    fn germinate<F: FnMut(&Wood)>(&self, shoot: &mut F) {
        shoot(&BodyBark(self.stmts.len()));
        for stmt in self.stmts.iter() {
            stmt.germinate(shoot);
        }
    }
}

////////////////////////////////////////////

#[allow(dead_code)]
fn write_png(body: &Body, filename: &str) {
    let mut img = image::ImageBuffer::new(170, 40);

    draw_math(body, &mut img);

    let ref mut fout = File::create(filename).unwrap();
    image::ImageLuma8(img).save(fout, image::PNG).unwrap();
}

/// Invokes ./refresh for shoddy livecoding.
#[allow(dead_code)]
fn change_desktop_background(filename: &str) {
    if let Ok(mut cmd) = Command::new("sh").arg("-c").arg("./refresh").arg(filename).spawn() {
        println!("Refreshing.");
        let _ = cmd.wait();
    }
    else {
        println!("Wrote {}.", filename);
    }
}

#[test]
fn test_png_output() {
    let filename = "out.png";
    write_png(&build_math(), filename);
    change_desktop_background(filename)
}

////////////////////////////////

#[cfg(not(test))]
fn main() {
    scene::main();
}
