use std::path::PathBuf;


error_chain! {
    foreign_links {
        FromUtf8(::std::string::FromUtf8Error);
        Ini(::ini::ini::Error);
        Io(::std::io::Error);
        Regex(::regex::Error);
        Reqwest(::reqwest::Error);
        Rusqlite(::rusqlite::Error);
    }

    errors {
        ParsePager(input: String) {
            description("cannot parse pager")
            display("cannot parse pager sourcerfrom '{}'", input)
        }

        ParseSource(input: String) {
            description("cannot parse source")
            display("cannot parse source from '{}'", input)
        }

        StdoutNoTermWidth {
            description("error while determining width of stdout terminal")
            display("error while determining width of stdout terminal")
        }

        NoCaptures {
            description("no captures foundi at all")
            display("no captures found at all")
        }

        NoCapturesIndex(index: usize) {
            description("no captures found at index")
            display("no captures found at index {}", index)
        }

        NoIndexDb {
            description("can't find index.db")
            display("can't find index.db")
        }

        NoDbConn {
            description("no Cppman::db_conn available!")
            display("no Cppman::db_conn available!")
        }

        Interrupted(msg: String) {
            description("keyboard interrupt")
            display("keyboard interrupt: '{}'", msg)
        }

        NoMatch(pattern: String) {
            description("no match")
            display("no match: '{}'", pattern)
        }

        WrongSource(source: String) {
            description("wrong source")
            display("wrong source: '{}'", source)
        }

        Abort(msg: String) {
            description("critical error, must abort")
            display("{}", msg)
        }

        NodeNotPresent(index: usize) {
            description("node not present in document")
            display("node #{} not present in document", index)
        }

        NotFilename(filename: PathBuf, msg: String) {
            description("not a filename")
            display("not a filename: '{:?}' ({})", filename, msg)
        }
    }
}
