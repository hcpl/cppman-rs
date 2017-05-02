use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::hash_map::Entry::Occupied;

use either::{Either, Left, Right};
use nom::{IResult, GetInput};
use regex::{Regex, Captures};

use ::utils::{HtmlError, repeat_char};


lazy_static! {
    static ref PSEUDO_NODE: Regex = Regex::new(
        "(?s)<\\s*([^/]\\w*)\\s?(.*?)>(.*?)<\\s*/([^/]\\w*).*?>").unwrap();
    static ref ATTR: Regex = Regex::new("(?x)
        \\s*
        (\\w+?)
        \\s* = \\s*
        (?: \' ((?:\\.|[^\'])*) \'
          | \" ((?:\\.|[^\"])*) \" )
    ").unwrap();
}

macro_rules! cond_else (
    ($i:expr, $cond:expr, $fmac:ident!( $($fargs:tt)* ), $gmac:ident!( $($gargs:tt)* )) => (
        {
            if $cond {
                map!($i, $fmac!($($fargs)*), Left)
            } else {
                map!($i, $gmac!($($gargs)*), Right)
            }
        }
    );
    ($i:expr, $cond:expr, $fmac:ident!( $($fargs:tt)* ), $g:expr) => (
        cond_else!($i, $cond, $fmac!($($fargs)*), call!($g))
    );
    ($i:expr, $cond:expr, $f:expr, $gmac:ident!( $($gargs:tt)* )) => (
        cond_else!($i, $cond, call!($f), $gmac!($($gargs)*))
    );
    ($i:expr, $cond:expr, $f:expr, $g:expr) => (
        cond_else!($i, $cond, call!($f), call!($g))
    );
);

fn is_alphanumeric_(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
}

fn is_not_less_than(c: char) -> bool {
    c != '<'
}


#[derive(Debug)]
struct Node {
    // Original code didn't even **use** parent!!
    //parent: Option<&'a Node<'a>>,
    name: String,
    //body: String,
    attr: HashMap<String, String>,
    text: String,
    children: RefCell<Vec<Node>>,
}

impl Node {
    fn new(name: &str, attr_list: &str, body: &str) -> Node {
        parse_node(name, attr_list, body)
    }

    fn text(&self) -> String {
        let mut s = String::new();
        self.impl_text(&mut s);
        s
    }

    fn impl_text(&self, out: &mut String) {
        out.push_str(&self.text);

        for c in self.children.borrow().iter() {
            c.impl_text(out);
        }
    }
}

fn parse_node(name: &str, attr_list: &str, body: &str) -> Node {
    let attr = ATTR.captures_iter(attr_list).map(|c| {
        if c.get(2) == None {
            (c[0].to_owned(), c[1].to_owned())
        } else {
            (c[0].to_owned(), c[2].to_owned())
        }
    }).collect::<HashMap<String, String>>();

    let text;
    let children;

    if name == "th" || name == "td" {
        text = strip_tags(Left(body));
        children = Vec::new();
    } else {
        text = "".to_owned();
        children = parse_children(body).to_full_result().unwrap_or(Vec::new());
    }

    Node {
        name: name.to_owned(),
        attr: attr,
        text: text,
        children: RefCell::new(children),
    }
}

fn parse_children(body: &str) -> IResult<&str, Vec<Node>> {
    let i = take_while!(body, char::is_whitespace).remaining_input().unwrap_or(body);

    many0!(i, do_parse!(
        char!('<') >>
        take_while!(char::is_whitespace) >>
        name: take_while!(is_alphanumeric_) >>
        take_while!(char::is_whitespace) >>
        attr: map!(many0!(do_parse!(
            key: take_while!(is_alphanumeric_) >>
            take_while!(char::is_whitespace) >>
            char!('=') >>
            take_while!(char::is_whitespace) >>
            quote_mark: one_of!("'\"") >>
            value: map!(many0!(none_of!(&quote_mark.to_string())), |v: Vec<_>| v.into_iter().collect::<String>()) >>
            char!(quote_mark) >>
            take_while!(char::is_whitespace) >>

            ((key.to_owned(), value.to_owned()))
        )), |v: Vec<_>| v.into_iter().collect::<HashMap<_, _>>()) >>
        char!('>') >>
        text: take_while!(is_not_less_than) >>
        children: parse_children >>
        char!('<') >>
        take_while!(char::is_whitespace) >>
        char!('/') >>
        tag!(name) >>
        take_while!(char::is_whitespace) >>
        char!('>') >>
        take_while!(char::is_whitespace) >>

        ({
            let text_;
            let children_;

            if name == "th" || name == "td" {
                text_ = format!("{}{}", text, children.iter().map(Node::text).collect::<String>());
                children_ = Vec::new();
            } else {
                text_ = text.to_owned();
                children_ = children;
            }

            Node {
                name: name.to_owned(),
                attr: attr,
                text: text_,
                children: RefCell::new(children_),
            }
        })
    ))
}


fn strip_tags(html: Either<&str, &Captures>) -> String {
    let data;

    match html {
        Left(s)  => data = s,
        Right(c) => data = &c[3],
    }

    PSEUDO_NODE.replace_all(data, |c: &Captures| strip_tags(Right(c))).into_owned()
}

fn traverse(node: &Node, depth: Option<usize>) {
    let depth = depth.unwrap_or(0);

    println!("{}{}: {:?} {}", repeat_char(' ', depth), node.name, node.attr, node.text);

    for c in node.children.borrow().iter() {
        traverse(&c, Some(depth + 2));
    }
}

fn get_row_width(node: &Node) -> Result<usize, HtmlError> {
    let mut total = 0;
    assert_eq!(node.name, "tr");

    for c in node.children.borrow().iter()  {
        if let Some(cspan) = c.attr.get("colspan") {
            total += try!(cspan.parse::<usize>().map_err(HtmlError::from_error));
        } else {
            total += 1
        }
    }

    Ok(total)
}

// Preserving the original naming, though this must be EXPAND_STR :)
#[cfg(target_os = "macos")]
const EXPAND_CHAR: &'static str = "";
#[cfg(not(target_os = "macos"))]
const EXPAND_CHAR: &'static str = "x";

fn scan_format(node: &Node,
               index: Option<usize>,
               width: Option<usize>,
               rowspan: &mut HashMap<usize, usize>) -> Result<String, HtmlError> {
    let index = index.unwrap_or(0);
    let mut width = width.unwrap_or(0);

    let mut format_str = String::new();

    if node.name == "th" || node.name == "td" {
        let extend = (width == 3 && index == 1) ||
                     (width != 3 && width < 5 && index == width - 1);

        if node.name == "th" {
            format_str.push_str(&format!("c{} ", if extend { EXPAND_CHAR } else { "" }));
        } else {
            format_str.push_str(&format!("l{} ", if extend { EXPAND_CHAR } else { "" }));
        }

        if let Some(cspan) = node.attr.get("colspan") {
            for _ in 0..(try!(cspan.parse::<usize>().map_err(HtmlError::from_error)) - 1) {
                format_str.push_str("s ");
            }
        }

        if let Some(rspan) = node.attr.get("rowspan") {
            let rspan = try!(rspan.parse::<usize>().map_err(HtmlError::from_error));
            if rspan > 1 {
                rowspan.insert(index, rspan - 1);
            }
        }
    }

    if node.name == "tr" && rowspan.len() > 0 {
        let mut ci = 0;

        for i in 0..width {
            if rowspan.contains_key(&i) {
                format_str.push_str("^ ");
                if rowspan[&i] == 1 {
                    rowspan.remove(&i);
                } else {
                    *rowspan.get_mut(&i).unwrap() -= 1;
                }
            } else {
                // There is a row span, but the current number of column is
                // not enough. Pad empty node when this happens.
                if ci >= node.children.borrow().len() {
                    node.children.borrow_mut().push(Node::new("td", "", ""));
                }

                format_str.push_str(&try!(scan_format(
                    &node.children.borrow()[ci], Some(i), Some(width), rowspan)));
                ci += 1;
            }
        }
    } else {
        if node.children.borrow().len() > 0 && node.children.borrow()[0].name == "tr" {
            width = try!(get_row_width(&node.children.borrow()[0]));
        }

        for (i, c) in node.children.borrow().iter().enumerate() {
            format_str.push_str(&try!(scan_format(&c, Some(i), Some(width), rowspan)));
        }
    }

    if node.name == "table" {
        format_str.push_str(".\n");
    } else if node.name == "tr" {
        format_str.push_str("\n");
    }

    Ok(format_str)
}

fn gen(node: &Node,
       output: &mut String,
       index: Option<usize>,
       last: Option<bool>,
       rowspan: &mut HashMap<usize, usize>) -> Result<(), HtmlError> {
    let index = index.unwrap_or(0);
    let last = last.unwrap_or(false);

    if node.name == "table" {
        let mut scan_format_rowspan = HashMap::new();

        output.push_str(".TS\n");
        output.push_str("allbox tab(|);\n");
        output.push_str(&try!(scan_format(node, None, None, &mut scan_format_rowspan)));
    } else if node.name == "th" || node.name == "td" {
        output.push_str(&format!("T{{\n{}", node.text));

        if let Some(rspan) = node.attr.get("rowspan") {
            let rspan = try!(rspan.parse::<usize>().map_err(HtmlError::from_error));
            if rspan > 1 {
                rowspan.insert(index, rspan - 1);
            }
        }
    } else {
        output.push_str(&node.text);
    }

    if node.name == "tr" && rowspan.len() > 0 {
        let total = rowspan.len() + node.children.borrow().len();
        let mut ci = 0;

        for i in 0..total {
            let mut has_entry = false;

            if let Occupied(mut rspan) = rowspan.entry(i) {
                output.push_str(&format!("\\^{}", if i < total - 1 { "|" } else { "" }));
                if *rspan.get() == 1 {
                    rspan.remove_entry();
                } else {
                    *rspan.get_mut() -= 1;
                }

                has_entry = true;
            }

            if !has_entry {
                // There is a row span, but the current number of column is
                // not enough. Pad empty node when this happens.
                if ci >= node.children.borrow().len() {
                    node.children.borrow_mut().push(Node::new("td", "", ""));
                }

                try!(gen(&node.children.borrow()[ci], output, Some(i), Some(i == total - 1), rowspan));
                ci += 1;
            }
        }
    } else {
        for (i, c) in node.children.borrow().iter().enumerate() {
            try!(gen(&c, output, Some(i), Some(i == node.children.borrow().len() - 1), rowspan));
        }
    }

    if node.name == "table" {
        output.push_str(".TE\n");
        output.push_str(".sp\n.sp\n");
    } else if node.name == "tr" {
        output.push_str("\n");
    } else if node.name == "th" || node.name == "td" {
        output.push_str(&format!("\nT}}{}", if !last { "|" } else { "" }))
    }

    Ok(())
}

pub fn parse_table(html: &str) -> Result<String, HtmlError> {
    let root = Node::new("root", "", html);
    let mut output = String::new();
    let mut gen_rowspan = HashMap::new();

    try!(gen(&root, &mut output, None, None, &mut gen_rowspan));
    Ok(output)
}
