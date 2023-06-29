use std::io::Write;

const ZENN_ENDPOINT: &str = "https://zenn.dev/api";

#[derive(serde::Deserialize)]
struct ScrapApiResult {
    pub scraps: Vec<ScrapInfo>,
    pub next_page: serde_json::Value,
}

#[derive(serde::Deserialize)]
struct ScrapInfo {
    pub slug: String,
}

#[derive(serde::Deserialize)]
struct ScrapContent {
    pub title: String,
    pub comments: Vec<ScrapComment>,
}
#[derive(serde::Deserialize)]
struct ScrapComment {
    pub author: String,
    pub created_at: String,
    pub body_markdown: String,
    pub children: Option<Vec<ScrapComment>>,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let offline = args.len() > 1 && args[1] == "--offline";

    let scraps = if offline {
        load_contents()
    } else {
        println!("ZennのCookieを入力してください: ");
        //read text from stdin
        let mut cookie = String::new();
        std::io::stdin()
            .read_line(&mut cookie)
            .expect("Failed to read line");

        let scraps_info = fetch_scrap_info(&cookie);
        fetch_contents(&cookie, scraps_info)
    };

    println!("{}件のscrapが見つかりました。", scraps.len());

    scraps.iter().for_each(|scrap| {
        let markdown = generate_markdown(&scrap.comments);
        let mut file = std::fs::File::create(format!(
            "markdown/{}.md",
            sanitize_filename::sanitize(&scrap.title)
        ))
        .expect("markdownファイルを作成できませんでした。");
        file.write_all(markdown.as_bytes())
            .expect("markdownファイルに書き込めませんでした。");
    });

    println!("完了しました。");
}

fn fetch_scrap_info(cookie: &str) -> Vec<ScrapInfo> {
    std::fs::create_dir_all("scraps").expect("出力先ディレクトリの作成に失敗しました。");
    let client = reqwest::blocking::Client::new();
    let mut page = 0;
    let mut scraps: Vec<ScrapInfo> = Vec::new();
    loop {
        let result = client
            .get(format!("{}/me/scraps?page={page}", ZENN_ENDPOINT))
            .header("Cookie", cookie.trim())
            .send()
            .expect("Request error")
            .json::<ScrapApiResult>()
            .expect("Failed to parse json");

        result.scraps.into_iter().for_each(|x| scraps.push(x));

        if result.next_page.is_null() {
            break;
        }
        page += 1;
    }

    scraps
}

fn fetch_contents(cookie: &str, scraps_info: Vec<ScrapInfo>) -> Vec<ScrapContent> {
    let client = reqwest::blocking::Client::new();
    scraps_info
        .iter()
        .enumerate()
        .map(|(i, scrap)| {
            let result = client
                .get(format!("{}/scraps/{}/blob.json", ZENN_ENDPOINT, scrap.slug))
                .header("Cookie", cookie.trim())
                .send()
                .expect("Request error")
                .text()
                .expect("Json parse error");

            let mut file = std::fs::File::create(format!("scraps/{}.json", scrap.slug))
                .expect("ファイルを作成できませんでした。");
            file.write_all(result.as_bytes())
                .expect("ファイルに書き込めませんでした。");
            println!("{} / {} を書き込みました。", i + 1, scraps_info.len());

            serde_json::from_str::<ScrapContent>(&result).expect("JSONのパースに失敗しました。")
        })
        .collect()
}

fn load_contents() -> Vec<ScrapContent> {
    std::fs::read_dir("scraps")
        .expect("scrapsディレクトリが見つかりませんでした。")
        .map(|x| {
            let path = x.expect("ディレクトリの読み込みに失敗しました。").path();
            let file = std::fs::File::open(path).expect("ファイルの読み込みに失敗しました。");
            serde_json::from_reader(file).expect("JSONのパースに失敗しました。")
        })
        .collect()
}

fn generate_markdown(comments: &[ScrapComment]) -> String {
    std::fs::create_dir_all("markdown").expect("出力先ディレクトリの作成に失敗しました。");
    let created_date = comments
        .first()
        .expect("コメントがありません。")
        .created_at
        .split('T')
        .next()
        .expect("日付の取得に失敗しました。");
    let updated_date = comments
        .iter()
        .map(|x| &x.created_at)
        .max()
        .expect("コメントがありません。");
    format!(
        "----\ncreated: {}\nupdated: {}\n----\n\n{}",
        created_date,
        updated_date,
        generate_markdown_content(comments, 0)
    )
}

fn generate_markdown_content(comments: &[ScrapComment], depth: usize) -> String {
    let mut result = String::new();
    for comment in comments {
        result.push_str(&format!(
            "{} {} by {}:\n\n{}\n\n",
            "#".repeat(depth + 1),
            comment.created_at,
            comment.author,
            comment
                .body_markdown
                .split('\n')
                .map(|x| {
                    let re = regex::Regex::new(r"(#+) (.*)").unwrap();
                    if let Some(cap) = re.captures(x) {
                        if let Some(headline) = cap.get(1) {
                            if let Some(content) = cap.get(2) {
                                return format!(
                                    "{} {}",
                                    "#".repeat(headline.as_str().len() + depth + 1),
                                    content.as_str()
                                );
                            }
                        }
                    };
                    x.to_string()
                })
                .collect::<Vec<String>>()
                .join("\n")
        ));
        if let Some(children) = &comment.children {
            result.push_str(&generate_markdown_content(children, depth + 1));
        }
    }
    result
}
