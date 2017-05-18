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

        NoIndexDb {
            description("can't find index.db")
            display("can't find index.db")
        }
    }
}
