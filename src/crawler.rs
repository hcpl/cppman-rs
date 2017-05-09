use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::thread;

use regex::Regex;
use reqwest::{self, Client, Response, StatusCode, IntoUrl};
use reqwest::header::Headers;
use url::{self, Url, Host};


pub struct Document<'a> {
    pub url: Url,
    query: String,
    status: StatusCode,
    pub text: String,
    headers: &'a Headers,
}

impl<'a> Document<'a> {
    pub fn new<T: IntoUrl>(response: Response, url: T) -> Document<'a> {
        let url =  url.into_url().unwrap();

        Document {
            url: url,
            query: url.query().unwrap_or("").to_owned(),
            status: *response.status(),
            text: {
                let mut text = String::new();
                response.read_to_string(&mut text);
                text
            },
            headers: response.headers(),
        }
    }
}


lazy_static! {
    static ref LINK: Regex = Regex::new("(?s)href\\s*=\\s*['\"]([^'\"]+)['\"]").unwrap();
}

enum FollowMode {
    Any, SameDomain, SameHost, SamePath,
}

pub struct Crawler {
    visited: HashSet<Url>,
    targets: HashSet<Url>,
}

impl Crawler {
    pub fn new() -> Crawler {
        Crawler {
            visited: HashSet::new(),
            targets: HashSet::new(),
        }
    }

    pub fn crawl<T: IntoUrl>(&self, url: T) -> reqwest::Result<()> {
        self.add_target(url);

        let client = try!(Client::new());

        while self.targets.len() > 0 {
            let url = self.targets.take(self.targets.iter().next().unwrap()).unwrap();

            let res = try!(client.get(url).send());
            if !res.status().is_success() {
                continue;
            }

            let mut text = String::new();
            res.read_to_string(&mut text);

            self.visited.insert(url);

            for link in LINK.captures_iter(&text).map(|cap| &cap[1]) {
                self.add_target(link);
            }
        }

        Ok(())
    }

    fn add_target<T: IntoUrl>(&self, url: T) -> Result<(), url::ParseError> {
        let url = try!(url.into_url());

        if !self.visited.contains(&url) {
            self.targets.insert(url);
        }

        Ok(())
    }
}


#[cfg(test)]
mod tests {
    #[test]
    fn test_crawl() {
        let url = "http://cplusplus.com/reference/";
        let crawler = ::crawler::Crawler::new();
        crawler.crawl(url);
        println!("{:?}", crawler.visited);
        println!("\n-------------------------------\n{:?}", crawler.targets);
    }
}
