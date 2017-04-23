use std::collections::HashMap;

use url::Url;


struct Document {
    pub url: Url,
    query: String,
    status: u32,
    pub text: String,
    headers: HashMap<String, String>,
}

impl Document {
    fn new(response: Response, url: Url) -> Document {
        Document {
            url: url,
            query: url.query().unwrap_or("").to_owned(),
            status: response.status,
            text: response.read(),
            headers: HashMap::new().extend(response.getheaders()),
        }
    }
}
