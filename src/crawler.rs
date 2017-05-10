use std::collections::HashSet;
use std::io::Read;

use ordermap::OrderMap;
use regex::Regex;
use reqwest::{self, Client, Response, StatusCode, IntoUrl};
use reqwest::header::{Headers, ContentType};
use url::{self, Url};


pub struct Document<'a> {
    pub url: Url,
    query: String,
    status: StatusCode,
    pub text: String,
    headers: &'a Headers,
}

impl<'a> Document<'a> {
    pub fn new<T: IntoUrl>(response:&'a mut Response, url: T) -> Document<'a> {
        let url =  url.into_url().unwrap();

        Document {
            url: url.clone(),
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
    targets: OrderMap<Url, ()>,
}

impl Crawler {
    pub fn new() -> Crawler {
        Crawler {
            visited: HashSet::new(),
            targets: OrderMap::new(),
        }
    }

    pub fn crawl<T: IntoUrl>(&mut self, url: T) -> reqwest::Result<()> {
        self.add_target(url);

        let client = try!(Client::new());

        while self.targets.len() > 0 {
            let (url, _) = self.targets.pop().unwrap();

            let mut res = try!(client.get(url.clone()).send());
            if !res.status().is_success() {
                let ct = res.headers().get::<ContentType>();
                if !ct.is_some() || equal_content_types(ct.unwrap(), &ContentType(mime!(Text/Html))) {
                    continue;
                }
            }

            let mut text = String::new();
            res.read_to_string(&mut text);

            self.visited.insert(url);

            for cap in LINK.captures_iter(&text) {
                self.add_target(&cap[1]);
            }
        }

        Ok(())
    }

    fn add_target<T: IntoUrl>(&mut self, url: T) -> Result<(), url::ParseError> {
        let url = try!(url.into_url());

        if !self.visited.contains(&url) {
            self.targets.insert(url, ());
        }

        Ok(())
    }
}

fn equal_content_types(ct1: &ContentType, ct2: &ContentType) -> bool {
    ct1.0 == ct2.0
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
