use std::io::Write;

const ZENN_API_URL: &str = "https://zenn.dev/api";

#[derive(serde::Deserialize)]
struct ScrapApiResult {
    pub scraps: Vec<Scrap>,
    pub next_page: serde_json::Value,
}

#[derive(serde::Deserialize)]
struct Scrap {
    pub slug: String,
}

fn main() {
    println!("ZennのCookieを入力してください: ");
    //read text from stdin
    let mut x = String::new();
    std::io::stdin()
        .read_line(&mut x)
        .expect("Failed to read line");

    let client = reqwest::blocking::Client::new();
    let mut page = 0;
    let mut slugs = Vec::new();
    loop {
        let result = client
            .get(format!("{}/me/scraps?page={page}", ZENN_API_URL))
            .header("Cookie", x.trim())
            .send()
            .expect("Request error")
            .json::<ScrapApiResult>()
            .expect("Failed to parse json");

        result.scraps.into_iter().for_each(|x| slugs.push(x.slug));

        if result.next_page.is_null() {
            break;
        }
        page += 1;
    }

    println!("{}件のscrapが見つかりました。", slugs.len());

    std::fs::create_dir_all("scraps").expect("出力先ディレクトリの作成に失敗しました。");

    for (i, slug) in slugs.iter().enumerate() {
        let result = client
            .get(format!("{}/scraps/{}/blob.json", ZENN_API_URL, slug))
            .header("Cookie", x.trim())
            .send()
            .expect("Request error")
            .text()
            .expect("Request error (text)");
        let mut file = std::fs::File::create(format!("scraps/{}.json", slug))
            .expect("ファイルを作成できませんでした。");
        file.write_all(result.as_bytes())
            .expect("ファイルに書き込めませんでした。");
        println!("{} / {} を書き込みました。", i + 1, slugs.len());
    }

    println!("完了しました。");
}
