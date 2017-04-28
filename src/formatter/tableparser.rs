use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::hash_map::Entry::Occupied;

use either::{Either, Left, Right};
use regex::{Regex, Captures, CaptureMatches, Replacer};

use ::formatter::utils::{HtmlError, repeat_char};


// Original regexes (which use backreferences):
// NODE = re.compile(r'<\s*([^/]\w*)\s?(.*?)>(.*?)<\s*/\1.*?>', re.S)
// ATTR = re.compile(r'\s*(\w+?)\s*=\s*([\'"])((?:\\.|(?!\2).)*)\2')

struct NodeRegex {
    regex: Regex,
}

struct NodeCaptureMatches<'r, 't> {
    cap_matches: CaptureMatches<'r, 't>,
}

impl NodeRegex {
    fn new() -> NodeRegex {
        NodeRegex { regex: Regex::new("(?s)<\\s*([^/]\\w*)\\s?(.*?)>(.*?)<\\s*/([^/]\\w*).*?>").unwrap() }
    }

    fn captures_iter<'r, 't>(&'r self, text: &'t str) -> NodeCaptureMatches<'r, 't> {
        NodeCaptureMatches { cap_matches: self.regex.captures_iter(text) }
    }

    fn replace_all<'t, R: Replacer>(&self, text: &'t str, mut rep: R) -> Cow<'t, str> {
        let mut it = self.captures_iter(text).peekable();
        if it.peek().is_none() {
            return Cow::Borrowed(text);
        }

        let mut new = String::with_capacity(text.len());
        let mut last_match = 0;
        for cap in it {
            if cap[1] == cap[4] {
                let m = cap.get(0).unwrap();
                new.push_str(&text[last_match..m.start()]);
                rep.replace_append(&cap, &mut new);
                last_match = m.end();
            }
        }

        new.push_str(&text[last_match..]);
        Cow::Owned(new)
    }
}

impl<'r, 't> Iterator for NodeCaptureMatches<'r, 't> {
    type Item = Captures<'t>;

    fn next(&mut self) -> Option<Captures<'t>> {
        loop {
            match self.cap_matches.next() {
                Some(cap) => if cap[1] == cap[4] { return Some(cap) },
                None      => return None,
            }
        }
    }
}


lazy_static! {
    static ref NODE: NodeRegex = NodeRegex::new();
    static ref ATTR: Regex = Regex::new("(?x)
        \\s*
        (\\w+?)
        \\s* = \\s*
        (?: \' ((?:\\.|[^\'])*) \'
          | \" ((?:\\.|[^\"])*) \" )
    ").unwrap();
}


#[derive(Debug)]
struct Node {
    // Original code didn't event **use** parent!!
    //parent: Option<&'a Node<'a>>,
    name: String,
    body: String,
    attr: HashMap<String, String>,
    text: String,
    children: RefCell<Vec<Node>>,
}

impl Node {
    fn new(name: &str, attr_list: &str, body: &str) -> Node {
        let attr = ATTR.captures_iter(attr_list).map(|c| {
            if c.get(2) == None {
                (c[0].to_owned(), c[1].to_owned())
            } else {
                (c[0].to_owned(), c[2].to_owned())
            }
        }).collect::<HashMap<String, String>>();

        let mut node = Node {
            name: name.to_owned(),
            body: body.to_owned(),
            attr: attr,
            text: "".to_owned(),
            children: RefCell::new(Vec::new()),
        };

        if name == "th" || name == "td" {
            node.text = strip_tags(Left(body));
        } else {
            node.children.borrow_mut().extend(
                NODE.captures_iter(body)
                    .map(|c| Node::new(&c[1], &c[2], &c[3])));
        }

        node
    }
}


fn strip_tags(html: Either<&str, &Captures>) -> String {
    let mut data;

    match html {
        Left(s)  => data = s,
        Right(c) => data = &c[3],
    }

    NODE.replace_all(data, |c: &Captures| strip_tags(Right(c))).into_owned()
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
