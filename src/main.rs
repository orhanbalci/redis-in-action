pub mod ch1;

fn main() {
    println!("Hello, world!");

    let client = redis::Client::open("redis://localhost:7001/");
    let mut con = client.unwrap().get_connection().unwrap();
    let res = ch1::post_article(&mut con, "orhan", "article 1", "http://article1").unwrap();
    println!("Posted article {:?}", res);
    let articles = ch1::get_articles(&mut con, 1, "score:").unwrap();
    articles.iter().for_each(|art| println!("{:?}", art));
}
