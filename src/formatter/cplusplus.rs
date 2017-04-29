use std::ops::Deref;

use regex::{Regex, Captures};
use chrono::Local;

use ::formatter::tableparser::parse_table;
use ::util::fixup_html;


lazy_static! {
    static ref PRE_RPS: Vec<(Regex, String)> = vec![
        (Regex::new("(?s)<table class=\"snippet\">(.*?)</table>").unwrap(),
         "\n.in +2n\n$1\n.in\n.sp\n".to_owned()),
    ];

    static ref PRE_SECTION: Regex = Regex::new("(?s)<pre.*?>(.*?)</pre>").unwrap();
    static ref TABLE: Regex = Regex::new("(?s)<table.*?>.*?</table>").unwrap();
    static ref ESCAPE_COLUMN: (Regex, String) =
        (Regex::new("(?s)T\\{\n(\\..*?)\nT\\}").unwrap(), "T{\n\\E $1\nT}".to_owned());

    static ref RPS: Vec<(Regex, String)> = vec![
        // Header, Name
        (Regex::new("(?s)\\s*<div id=\"I_type\"[^>]*>(.*?)\\s*</div>\\s*\
                     <div id=\"I_file\"[^>]*>(.*?)</div>\\s*\
                     <h1>(.*?)</h1>\\s*<div class=\"C_prototype\"[^>]*>\
                     (.*?)</div>\\s*<div id=\"I_description\"[^>]*>(.*?)</div>").unwrap(),
         format!(".TH \"$3\" 3 \"{}\" \"cplusplus.com\" \"C++ Programmer\\'s Manual\"\n\
                  \n.SH \"NAME\"\n$3 - $5\n\
                  \n.SE\n.SH \"TYPE\"\n$1\n\
                  \n.SE\n.SH \"SYNOPSIS\"\n#include $2\n.sp\n$4\n\
                  \n.SE\n.SH \"DESCRIPTION\"\n", Local::today().naive_local())),
        (Regex::new("(?s)\\s*<div id=\"I_type\"[^>]*>(.*?)\\s*</div>\\s*\
                     <div id=\"I_file\"[^>]*>(.*?)</div>\\s*\
                     <h1>(.*?)</h1>\\s*\
                     <div id=\"I_description\"[^>]*>(.*?)</div>").unwrap(),
         format!(".TH \"$3\" 3 \"{}\" \"cplusplus.com\" \"C++ Programmer\\'s Manual\"\n\
                  \n.SH \"NAME\"\n$3 - $4\n\
                  \n.SE\n.SH \"TYPE\"\n$1\n\
                  \n.SE\n.SH \"SYNOPSIS\"\n#include $2\n.sp\n\
                  \n.SE\n.SH \"DESCRIPTION\"\n", Local::today().naive_local())),
        (Regex::new("(?s)\\s*<div id=\"I_type\"[^>]*>(.*?)\\s*</div>\\s*<h1>(.*?)</h1>\\s*\
                     <div id=\"I_description\"[^>]*>(.*?)</div>").unwrap(),
         format!(".TH \"$2\" 3 \"{}\" \"cplusplus.com\" \"C++ Programmer\\'s Manual\"\n\
                  \n.SH \"NAME\"\n$2 - $3\n\
                  \n.SE\n.SH \"TYPE\"\n$1\n\
                  \n.SE\n.SH \"DESCRIPTION\"\n", Local::today().naive_local())),
        (Regex::new("(?s)\\s*<div id=\"I_type\"[^>]*>(.*?)\\s*</div>\\s*<h1>(.*?)</h1>\\s*\
                     <div id=\"I_file\"[^>]*>(.*?)</div>\\s*<div id=\"I_description\"[^>]*>\
                     (.*?)</div>").unwrap(),
         format!(".TH \"$2\" 3 \"{}\" \"cplusplus.com\" \"C++ Programmer\\'s Manual\"\n\
                  \n.SH \"NAME\"\n$2 - $4\n\
                  \n.SE\n.SH \"TYPE\"\n$1\n\
                  \n.SE\n.SH \"DESCRIPTION\"\n", Local::today().naive_local())),
        (Regex::new("(?s)\\s*<div id=\"I_type\"[^>]*>(.*?)\\s*</div>\\s*<h1>(.*?)</h1>\\s*\
                     <div class=\"C_prototype\"[^>]*>(.*?)</div>\\s*\
                     <div id=\"I_description\"[^>]*>(.*?)</div>").unwrap(),
         format!(".TH \"$2\" 3 \"{}\" \"cplusplus.com\" \"C++ Programmer\\'s Manual\"\n\
                  \n.SH \"NAME\"\n$2 - $4\n\
                  \n.SE\n.SH \"TYPE\"\n$1\n\
                  \n.SE\n.SH \"SYNOPSIS\"\n$3\n\
                  \n.SE\n.SH \"DESCRIPTION\"\n", Local::today().naive_local())),
        (Regex::new("(?s)<span alt=\"[^\"]*?\" class=\"C_ico cpp11warning\"[^>]*>").unwrap(),
         " [C++11]".to_owned()),
        // Remove empty #include
        (Regex::new("#include \n.sp\n").unwrap(), "".to_owned()),
        // Remove empty sections
        (Regex::new("\n.SH (.+?)\n+.SE").unwrap(), "".to_owned()),
        // Section headers
        (Regex::new(".*<h3>(.+?)</h3>").unwrap(), "\n.SE\n.SH \"$1\"\n".to_owned()),
        // 'ul' tag
        (Regex::new("<ul>").unwrap(), "\n.RS 2\n".to_owned()),
        (Regex::new("</ul>").unwrap(), "\n.RE\n.sp\n".to_owned()),
        // 'li' tag
        (Regex::new("(?s)<li>\\s*(.+?)</li>").unwrap(), "\n.IP \\[bu] 3\n$1\n".to_owned()),
        // 'pre' tag
        (Regex::new("(?s)<pre[^>]*>(.+?)</pre\\s*>").unwrap(), "\n.nf\n$1\n.fi\n".to_owned()),
        // Subsections
        (Regex::new("<b>(.+?)</b>:<br/>").unwrap(), ".SS $1\n".to_owned()),
        // Member functions / See Also table
        // Without C++11 tag
        (Regex::new("(?s)<dl class=\"links\"><dt><a href=\"[^\"]*\"><b>([^ ]+?)</b></a></dt><dd>\
                     ([^<]*?)<span class=\"typ\">\\s*\\(([^<]*?)\n?\\)</span></dd></dl>").unwrap(),
         "\n.IP \"$1(3)\"\n$2 ($3)\n".to_owned()),
        // With C++11 tag
        (Regex::new("(?s)<dl class=\"links\"><dt><a href=\"[^\"]*\"><b>([^ ]+?) <b class=\"C_cpp11\" \
                     title=\"(.+?)\"></b></b></a></dt><dd>\
                     ([^<]*?)<span class=\"typ\">\\s*\\((.*?)\n?\\)</span></dd></dl>").unwrap(),
         "\n.IP \"$1(3) [$2]\"\n$3 ($4)\n".to_owned()),
        // Footer
        (Regex::new("(?s)<div id=\"CH_bb\">.*$").unwrap(),
         "\n.SE\n.SH \"REFERENCE\"\n\
          cplusplus.com, 2000-2015 - All rights reserved.".to_owned()),
        // C++ version tag
        (Regex::new("<div.+?title=\"(C\\+\\+..)\"[^>]*>").unwrap(), ".sp\n$1\n".to_owned()),
        // 'br' tag
        (Regex::new("<br/>").unwrap(), "\n.br\n".to_owned()),
        (Regex::new("\n.br\n.br\n").unwrap(), "\n.sp\n".to_owned()),
        // 'dd' 'dt' tag
        (Regex::new("(?s)<dt>(.+?)</dt>\\s*<dd>(.+?)</dd>").unwrap(), ".IP \"$1\"\n$2\n".to_owned()),
        // Bold
        (Regex::new("<strong>(.+?)</strong>").unwrap(), "\n.B $1\n".to_owned()),
        // Remove row number in EXAMPLE
        (Regex::new("(?s)<td class=\"rownum\">.*?</td>").unwrap(), "".to_owned()),
        // Any other tags
        (Regex::new("<script[^>]*>[^<]*</script>").unwrap(), "".to_owned()),
        (Regex::new("(?s)<.*?>").unwrap(), "".to_owned()),
        // Misc
        (Regex::new("&lt;").unwrap(), "<".to_owned()),
        (Regex::new("&gt;").unwrap(), ">".to_owned()),
        (Regex::new("&quot;").unwrap(), "\"".to_owned()),
        (Regex::new("&amp;").unwrap(), "&".to_owned()),
        (Regex::new("&nbsp;").unwrap(), " ".to_owned()),
        (Regex::new("\\\\([^\\^nE])").unwrap(), "\\\\$1".to_owned()),
        (Regex::new(">/\">").unwrap(), "".to_owned()),
        (Regex::new("/\">").unwrap(), "".to_owned()),
        // Remove empty lines
        (Regex::new("\n\\s*\n+").unwrap(), "\n".to_owned()),
        (Regex::new("\n\n+").unwrap(), "\n".to_owned()),
        // Preserve \n" in EXAMPLE
        //(Regex::new("\\\\n").unwrap(), "\\en".to_owned()),
    ];

    static ref SECTION_HEADER: Regex = Regex::new(".SH .*\n").unwrap();

    static ref PAGE_TYPE: Regex = Regex::new("\n\\.SH \"TYPE\"\n(.+?)\n").unwrap();
    static ref CLASS_NAME: Regex = Regex::new("\n\\.SH \"NAME\"\n(?:.*::)?(.+?) ").unwrap();
    static ref SECS: Regex = Regex::new("(?s)\n\\.SH \"(.+?)\"(.+?)\\.SE").unwrap();

    static ref MEMBER_NONAME: Regex = Regex::new("\n\\.IP \"([^:]+?)\"").unwrap();
    static ref MEMBER_CONSTRUCTOR: Regex = Regex::new("\\(constructor\\)").unwrap();
    static ref MEMBER_DESTRUCTOR: Regex = Regex::new("\\(destructor\\)").unwrap();

    static ref INHERITED_INHERIT: Regex = Regex::new(".+?INHERITED FROM (.+)").unwrap();
    static ref INHERITED_NONAME: Regex = Regex::new("\n\\.IP \"(.+)\"").unwrap();
}


/// Escape <pre> seciton in table.
fn escape_pre_section(table: &str) -> String {
    PRE_SECTION.replace_all(table, |c: &Captures| c[1].replace("\n", "\n.br\n")).into_owned()
}

/// Convert HTML text from cplusplus.com to Groff-formated text.
fn html2groff(data: &str, _name: &str) -> String {
    let mut data = data.to_owned();

    // Remove sidebar
    if let Some(pos) = data.find("<div class=\"C_doc\">") {
        data = data[pos..].to_owned();
    }

    // Pre replace all
    for &(ref reg, ref repl) in PRE_RPS.iter() {
        data = reg.replace_all(&data, repl.as_str()).into_owned();
    }

    let (ref ec_reg, ref ec_repl) = *ESCAPE_COLUMN.deref();
    data = TABLE.replace_all(&data, |c: &Captures| {
        let mut parsed_table = parse_table(&escape_pre_section(&c[0])).unwrap();
        parsed_table = ec_reg.replace_all(&parsed_table, ec_repl.as_str()).into_owned();
        parsed_table
    }).into_owned();

    // Replace all
    for &(ref reg, ref repl) in RPS.iter() {
        data = reg.replace_all(&data, repl.as_str()).into_owned();
    }

    // Upper case all section headers
    data = SECTION_HEADER.replace_all(&data, |c: &Captures| c[0].to_uppercase()).into_owned();

    // Quote from the original cppman:
    //
    // # Add tags to member/inherited member functions
    // # e.g. insert -> vector::insert
    //
    // # .SE is a pseudo macro I created which means 'SECTION END'
    // # The reason I use it is because I need a marker to know where section
    // # ends.
    // # re.findall find patterns which does not overlap, which means if I do
    // # this: secs = re.findall(r'\n\.SH "(.+?)"(.+?)\.SH', data, re.S)
    // # re.findall will skip the later .SH tag and thus skip the later section.
    // # To fix this, '.SE' is used to mark the end of the section so the next
    // # '.SH' can be find by re.findall

    let data_clone = data.clone();
    if let Some(page_type) = PAGE_TYPE.captures(&data_clone) {
        if page_type[1].contains("class") {
            let class_name = &CLASS_NAME.captures(&data_clone).unwrap()[1];
            let secs = SECS.captures_iter(&data_clone);

            for capture in secs {
                let (sec, content) = (&capture[1], &capture[2]);

                // Member functions
                if sec.contains("MEMBER") &&
                        !sec.contains("NOT-MEMBER") &&
                        !sec.contains("INHERITED") &&
                        sec != "MEMBER TYPES" {
                    let mut content2 = MEMBER_NONAME.replace_all(content,
                        format!("\n.IP \"{}::$1\"", content).as_str()).into_owned();
                    // Replace (constructor) (destructor)
                    content2 = MEMBER_CONSTRUCTOR.replace_all(&content2,
                        format!("{}", class_name).as_str()).into_owned();
                    content2 = MEMBER_DESTRUCTOR.replace_all(&content2,
                        format!("~{}", class_name).as_str()).into_owned();
                    data = data.replace(content, &content2);
                // Inherited member functions
                } else if sec.contains("MEMBER") && sec.contains("INHERITED") {
                    let inherit = INHERITED_INHERIT.captures(sec).unwrap()[1].to_lowercase();
                    let content2 = INHERITED_NONAME.replace_all(content,
                        format!("\n.IP \"{}::$1\"", content).as_str()).into_owned();
                    data = data.replace(content, &content2);
                }
            }
        }
    }

    // Remove pseudo macro '.SE'
    data = data.replace("\n.SE", "");

    data
}


#[test]
/// Test if there is major format changes in cplusplus.com
fn func_test() {
    let mut text = String::new();
    reqwest::get("http://www.cplusplus.com/printf").unwrap().read_to_string(&mut text).unwrap();
    let result = html2groff(&fixup_html(&text), "printf");

    assert!(result.contains(".SH \"NAME\""));
    assert!(result.contains(".SH \"TYPE\""));
    assert!(result.contains(".SH \"DESCRIPTION\""));
}

#[test]
/// Simple Text
fn test() {
    let mut text = String::new();
    reqwest::get("http://www.cplusplus.com/vector").unwrap().read_to_string(&mut text).unwrap();
    print!("{}", html2groff(&fixup_html(&text), "vector"));
}
