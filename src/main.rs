extern crate image;
#[macro_use]
extern crate log;

use std::cmp;
use std::fmt;
use std::fs::File;
use std::marker::PhantomData;
use std::process::Command;

use presses::{Paper, Press};

mod presses;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Size(u32, u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Pos(i32, i32);


fn draw_math<I: Paper>(paper: &mut I) {
    use Expr::*;

    let v = Var;
    let bind_v = Bind(Ref::new(0));
    let defn = Stmt::Let(v, Expr::Int(1));
    let minus = Minus(Box::new(Int(2)), Box::new(bind_v));
    let math = Plus(Box::new(Int(4)), Box::new(minus));
    let body = Body { stmts: vec![defn, Stmt::Print(math)] };

    let (branches, tokens) = grow_tree(&body);
    let tree = Tree::new(&branches[..]);
    draw_tree(&tree, &tokens[..], paper);
}

fn draw_tree<I: Paper>(tree: &Tree, tokens: &[String], paper: &mut I) {
    let ref press = presses::FreeTypePress::new().unwrap();

    const N: usize = 10;
    assert_eq!(tree.len(), N);
    assert_eq!(tokens.len(), N);

    let mut c_size = [Size(0, 0); N];
    compute_sizes(&tree, &mut c_size);

    let mut c_pos = [Pos(0, 0); N];
    compute_positions(&tree, &c_size, &mut c_pos);

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

fn compute_sizes(tree: &Tree, sizes: &mut [Size]) {
    assert!(sizes.len() >= tree.len(), "Not enough Sizes allocated");

    // measure everything, starting bottom-up
    // currently nothing fancy like margins or padding
    let _total_size = tree.flow_up(|ix, child_sizes| -> Size {

        // compute this from content
        let my_size = Size(10, 10);

        // size is sum of widths and max of heights
        let size = child_sizes.iter().fold(my_size, |total, child: &Size| {
            let sum_w = child.0 + total.0;
            let max_h = cmp::max(child.1, total.1);
            Size(sum_w, max_h)
        });
        sizes[ix] = size;
        size
    });
}

fn compute_positions(tree: &Tree, sizes: &[Size], coords: &mut [Pos]) {
    assert!(sizes.len() >= tree.len(), "Not enough Sizes allocated");
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
                // hack: move cursor past the content of this node
                child_pos.0 += 10;
                Some((child_pos, n))
            }
            else {
                None
            };

            // if we're done at this level, pop back out to our old cursor
            let size = sizes[ix];
            cursor.0 += size.0 as i32;
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
    Bind(Ref<Var>),
    Int(i32),
    Minus(Box<Expr>, Box<Expr>),
    Plus(Box<Expr>, Box<Expr>),
}

pub enum ExprBark {
    Bind(Ref<Var>),
    Int(i32),
    Minus,
    Plus,
}

impl Wood for ExprBark {
    fn branching_factor(&self) -> usize {
        use ExprBark::*;
        match *self {
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

fn main() {
    let mut img = image::ImageBuffer::new(100, 40);

    draw_math(&mut img);

    let filename = "out.png";
    let ref mut fout = File::create(filename).unwrap();
    image::ImageLuma8(img).save(fout, image::PNG).unwrap();

    change_desktop_background(filename)
}

/// Invokes ./refresh for shoddy livecoding.
fn change_desktop_background(filename: &str) {
    if let Ok(mut cmd) = Command::new("sh").arg("-c").arg("./refresh").arg(filename).spawn() {
        println!("Refreshing.");
        let _ = cmd.wait();
    }
    else {
        println!("Wrote {}.", filename);
    }
}
