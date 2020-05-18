use redis::Commands;
use redis::Connection;
use std::collections::HashMap;
use std::error::Error;
use std::time::Duration;
use std::time::SystemTime;

const ONE_WEEK_IN_SECONDS: u64 = 7 * 86400;
const VOTE_SCORE: u64 = 432;
const ARTICLE_PER_PAGE: u8 = 25;

pub fn article_vote(
    conn: &mut Connection,
    user: &str,
    article: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    let cutoff = SystemTime::now()
        .checked_sub(Duration::new(ONE_WEEK_IN_SECONDS, 0))
        .expect("Unexpected time value");
    let score: u64 = conn.zscore("time:", article)?;
    if score < cutoff.duration_since(SystemTime::UNIX_EPOCH)?.as_secs() {
        return Ok(false);
    }
    let article_id: u64 = article.split(':').last().unwrap().parse::<u64>()?;
    let is_added: u64 = conn.sadd(format!("voted:{}", article_id), user)?;

    if is_added == 1 {
        let _: u64 = conn.zincr("score:", article, VOTE_SCORE)?;
        let _: u64 = conn.hincr(article, "votes", 1)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub fn post_article(
    conn: &mut Connection,
    user: &str,
    title: &str,
    link: &str,
) -> Result<u64, Box<dyn Error>> {
    let article_id: u64 = conn.incr("article:", 1)?;
    conn.sadd(format!("voted:{}", article_id), user)?;
    conn.expire(
        format!("voted:{}", article_id),
        ONE_WEEK_IN_SECONDS as usize,
    )?;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs();
    conn.hset_multiple(
        format!("article:{}", article_id),
        &[
            ("title", title),
            ("link", link),
            ("poster", user),
            ("time", &now.to_string()),
            ("votes", &1.to_string()),
        ],
    )?;
    conn.zadd(
        "score:",
        format!("article:{}", article_id),
        now + VOTE_SCORE,
    )?;
    conn.zadd("time:", format!("article:{}", article_id), now)?;
    Ok(article_id)
}

pub fn get_articles(
    conn: &mut Connection,
    page: u8,
    order: &str,
) -> Result<Vec<HashMap<String, String>>, Box<dyn Error>> {
    let start = (page - 1) * ARTICLE_PER_PAGE;
    let end = start + ARTICLE_PER_PAGE - 1;
    let ids: Vec<String> = conn.zrevrange(order, start as isize, end as isize)?;
    Ok(ids
        .iter()
        .map(|id| conn.hgetall(id).unwrap())
        .collect::<Vec<_>>())
}

pub fn add_remove_groups(
    conn: &mut Connection,
    article_id: u64,
    to_add: Vec<&str>,
    to_remove: Vec<&str>,
) -> Result<bool, Box<dyn Error>> {
    let article = format!("article:{}", article_id);
    to_remove.iter().for_each(|&rem| {
        let _: u8 = conn.srem(format!("group:{}", rem), &article).unwrap();
    });
    to_add.iter().for_each(|&add| {
        let _: u8 = conn.sadd(format!("group:{}", add), &article).unwrap();
    });

    Ok(true)
}

pub fn get_group_articles(
    conn: &mut Connection,
    group: &str,
    page: u8,
    order: &str,
) -> Result<Vec<HashMap<String, String>>, Box<dyn Error>> {
    let key = format!("{}{}", order, group);
    if !conn.exists(&key)? {
        conn.zinterstore_max(key.as_str(), &[&format!("group:{}", group), order])?;
        conn.expire(&key, 60)?;
    }
    get_articles(conn, page, &key)
}
