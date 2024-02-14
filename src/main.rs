use collapse::collapse;
use std::env; 
use serenity::{http::Http, model::webhook::Webhook, builder::ExecuteWebhook, builder::CreateEmbed};

struct Manga {
    url: String,
    title: String,
    subtitle: String,
    date: String,
    cover_image: Option<String>,
}

fn parse_main_page(document: &scraper::Html) -> Option<Vec<Manga>>{
    let manga_selector = scraper::Selector::parse("div.bg-card").unwrap();
    let manga_blocks = document.select(&manga_selector);
    let mut manga_vec: Vec<Manga> = Vec::new();
    for manga in manga_blocks{
        let url = manga
            .select(&scraper::Selector::parse("a").unwrap())
            .next()
            .and_then(|img| img.value().attr("href"))
            .map(str::to_owned);

        //let full_url = String::new();
        let url = match url {
            Some(url) => "https://tcbscans.com".to_owned() + &url.to_owned(),
            _ => String::new(),
        };

        let title_block = manga
            .select(&scraper::Selector::parse("div").unwrap())
            .next()
            .map(|mange_title_block | mange_title_block.text().collect::<String>());
        
        let mut title_block_items: Vec<String> = Vec::new();
        if let Some(inner_text) = title_block {
            let inner_text = collapse(&inner_text
                .replace("\n", ";")
                .replace("   ", ""));
            for block in inner_text.split(";"){
                if block != "" && block != " " {
                    title_block_items.push(block.trim().to_string());
                }
            }
        } else {
            continue;
        }
        
        if title_block_items.len() == 3 {
            manga_vec.push(Manga {
                url: url,
                title: title_block_items[0].clone(),
                subtitle: title_block_items[1].clone(),
                date: title_block_items[2].clone(),
                cover_image: None
            })
        }
    };

    if manga_vec.len() == 0 {
        return None;
    }
    Some(manga_vec)
}

fn get_cover(document: &scraper::Html) -> Option<String> {
    
    let image_selector = scraper::Selector::parse("picture").unwrap();
    let image_blocks = document.select(&image_selector);
    
    let mut pages_vec: Vec<String> = Vec::new();
    for image in image_blocks {
        let image_url = image.select(&scraper::Selector::parse("img").unwrap())
            .next()
            .and_then(|img| img.value().attr("src"))
            .map(str::to_owned);
        if let Some(image_url) = image_url {
            pages_vec.push(image_url);
        }
    };

    if pages_vec.len() == 0 {
        return None;
    }
    Some(pages_vec[0].clone())
}

fn build_embed(manga: &Manga) -> Option<CreateEmbed> {
    let mut embed_title = String::new();
    if manga.title.contains("One Piece Chapter") {
        embed_title.push_str("Yohoho! A new chapter has released!");
    } else if manga.title.contains("Jujutsu Kaisen") {
        embed_title.push_str("Nah, I'd chapter.");
    } else {
        return None
    }

    let cover_image = match manga.cover_image.clone() {
        Some(cover_image ) => cover_image,
        _ => String::from("https://prodimage.images-bn.com/pimages/9780316073905_p0_v1_s1200x630.jpg"),
    };

    let subtitle = format!("Title: `{}`", manga.subtitle);
    let date: String = format!("Date: `{}`", manga.date);
    let url: String = format!("Link: [TCBScans.com]({})", manga.url);
    let embed = CreateEmbed::new()
        .title(embed_title)
        .description(&manga.title)
        .field("", &subtitle, false)
        .field("", &date, false)
        .field("", &url, false)
        .image(cover_image);

    return Some(embed)
}

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let key = "DISCORD_WEBHOOK";
    let discord_webhook = match env::var(key) {
        Ok(val) => val,
        Err(_) => panic!("Please specify a webhook in environment variables as DISCORD_WEBHOOK"),
    };
    let http = Http::new("");
    let webhook = Webhook::from_url(&http, &discord_webhook)
        .await
        .expect("Please provide a webhook URL.");
    
    let response: String = reqwest::get("https://tcbscans.com")
        .await?
        .text()
        .await?;
    let document = scraper::Html::parse_document(&response);
    
    let manga_vec = match parse_main_page(&document) {
        Some(vec) => vec,
        _ => panic!("Manga not found on page"),
    };

    for mut manga in manga_vec {
        // Grab and set the manga's cover image
        let response: String = reqwest::get(&manga.url)
            .await?
            .text()
            .await?;
        let document = scraper::Html::parse_document(&response);
        let cover_image = get_cover(&document);
        manga.cover_image = cover_image;

        let embed: CreateEmbed = match build_embed(&manga) {
            Some(embed) => embed,
            _ => continue,
        };

        let builder = ExecuteWebhook::new().embed(embed);
        webhook.execute(&http, false, builder)
            .await
            .expect("Could not execute webhook.");
    }

    Ok(())
}
