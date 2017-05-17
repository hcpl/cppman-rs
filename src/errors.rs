error_chain! {
    foreign_links {
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
    }
}
