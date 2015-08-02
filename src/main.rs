extern crate image;
#[macro_use]
extern crate log;

use std::cmp;
use std::fmt;
use std::fs::File;
use std::process::Command;

use presses::{Paper, Press};

mod presses;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Size(u32, u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Pos(i32, i32);


fn draw_math<I: Paper>(paper: &mut I) {
    let minus = Expr::Minus(Box::new(Expr::Int(2)), Box::new(Expr::Int(1)));
    let math = Expr::Plus(Box::new(Expr::Int(4)), Box::new(minus));

    let (branches, tokens) = grow_tree(&math);
    let tree = Tree::new(&branches[..]);
    draw_tree(&tree, &tokens[..], paper);
}

fn draw_tree<I: Paper, D: fmt::Display>(tree: &Tree, tokens: &[D], paper: &mut I) {
    let ref press = presses::FreeTypePress::new().unwrap();

    const N: usize = 5;
    assert!(tokens.len() >= N);

    let mut c_size = [Size(0, 0); N];
    compute_sizes(&tree, &mut c_size);

    let mut c_pos = [Pos(0, 0); N];
    compute_positions(&tree, &c_size, &mut c_pos);

    for ix in 0..N {
        let Pos(x, y) = c_pos[ix];
        // blit_str should take Write or something to avoid temporary
        let ref s = format!("{}", tokens[ix]);
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

/// Anything that can germinate a Tree of Ts.
pub trait Seed<T> {
    fn germinate<F: FnMut(Branch, T)>(&self, shoot: &mut F);
}

fn grow_tree<T, S: Seed<T>>(seed: &S) -> (Vec<Branch>, Vec<T>) {
    let mut branches = vec![];
    let mut tokens = vec![];
    seed.germinate(&mut |branch, token| {
        branches.push(branch);
        tokens.push(token);
    });
    (branches, tokens)
}

enum Expr {
    Int(i32),
    Minus(Box<Expr>, Box<Expr>),
    Plus(Box<Expr>, Box<Expr>),
}

enum ExprBark {
    Int(i32),
    Minus,
    Plus,
}

impl fmt::Display for ExprBark {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ExprBark::*;
        match *self {
            Int(i) => write!(f, "{}", i),
            Minus => write!(f, "-"),
            Plus => write!(f, "+"),
        }
    }
}

// ought to be auto-derived
impl Seed<ExprBark> for Expr {
    fn germinate<F: FnMut(Branch, ExprBark)>(&self, shoot: &mut F) {
        use Expr::*;
        match *self {
            Int(i) => shoot(Branch(0), ExprBark::Int(i)),
            Minus(ref left, ref right) => {
                shoot(Branch(2), ExprBark::Minus);
                left.germinate(shoot);
                right.germinate(shoot);
            }
            Plus(ref left, ref right) => {
                shoot(Branch(2), ExprBark::Plus);
                left.germinate(shoot);
                right.germinate(shoot);
            }
        }
    }
}

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
