#![feature(plugin)]
#![feature(custom_derive)]
#![plugin(rocket_codegen)]

extern crate select;
extern crate reqwest;
extern crate rocket;
extern crate rocket_contrib;
#[macro_use] extern crate serde_derive;

use std::collections::HashMap;
use rocket_contrib::Json;
use select::document::Document;
use select::predicate::{Attr, Class, Name, Predicate};

#[derive(FromForm)]
struct Query {
    q: String,
}

#[derive(Serialize)]
struct ReviewsSummary {
    source: String,
    url: String,
    strain: String,
    rating: f64,
    ratings: u32,
}

#[derive(Serialize)]
struct QueryResult {
    strain_reviews: HashMap<String, Vec<ReviewsSummary>>,
}

#[get("/strain/search?<q>")]
fn index(q: Query) -> Json<QueryResult> {
    let query = q.q.split_whitespace().collect::<Vec<&str>>();
    let leafly_revs = leafly(&query);
    let allbud_revs = allbud(&query);
    let mut revs = HashMap::new();
    for rev_summary in allbud_revs {
        revs.entry(rev_summary.strain.clone()).or_insert(Vec::new()).push(rev_summary);
    }
    for rev_summary in leafly_revs {
        revs.entry(rev_summary.strain.clone()).or_insert(Vec::new()).push(rev_summary);
    }
    for (strain, rev_sums) in &mut revs {
        if rev_sums.len() > 1 {
            let mut total_rating: f64 = 0.0;
            let mut total_ratings: u32 = 0;
            for rev_sum in rev_sums.iter() {
                total_rating += rev_sum.rating * (rev_sum.ratings as f64);
                total_ratings += rev_sum.ratings;
            }
            rev_sums.push(ReviewsSummary {
                source: "Meta Chronic Average".to_string(),
                url: String::new(),
                strain: strain.clone(),
                rating: total_rating / (total_ratings as f64),
                ratings: total_ratings,
            });
        }
    }
    Json(QueryResult {
        strain_reviews: revs
    })
}

fn allbud(search_terms: &Vec<&str>) -> Vec<ReviewsSummary> {
    let base_url = "https://www.allbud.com";
    let search_url = format!("{}{}{}", base_url, "/marijuana-strains/search?q=", search_terms.join("+"));
    let search_resp = reqwest::get(&search_url).unwrap();
    assert!(search_resp.status().is_success());

    let mut strain_urls = Vec::new();
    Document::from_read(search_resp).unwrap().find(Class("object-title")).for_each(|node| {
        let a_tags = node.find(Name("a"));
        let search_strains = a_tags.map(|tag| format!("{}{}", base_url, tag.attr("href").unwrap()));
        let filtered_strain_urls = search_strains.filter(|strain| {
            let mut contains_terms = true;
            search_terms.iter().for_each(|term| {
                contains_terms &= strain.contains(term);
            });
            return contains_terms;
        });
        filtered_strain_urls.for_each(|url| strain_urls.push(url));
    });

    let mut review_summaries = Vec::new();
    for url in strain_urls {
        let strain_resp = reqwest::get(&url).unwrap();
        assert!(strain_resp.status().is_success());

        let doc = Document::from_read(strain_resp).unwrap();
        let rating = doc.find(Class("rating-num")).next().unwrap();
        let num_ratings = doc.find(Attr("id", "product-rating-votes")).next().unwrap();
        let split_url: Vec<&str> = url.split('/').collect();
        let name = split_url.last().unwrap().replace("-", " ");
        review_summaries.push(ReviewsSummary {
            source: "Allbud".to_string(),
            url: url.clone(),
            strain: name.clone(),
            rating: rating.inner_html().trim().parse::<f64>().unwrap(),
            ratings: num_ratings.inner_html().trim().parse::<u32>().unwrap(),
        });
    }
    review_summaries
}

fn leafly(search_terms: &Vec<&str>) -> Vec<ReviewsSummary> {
    let base_url = "https://www.leafly.com";
    let search_url = format!("{}{}{}{}", base_url, "/search?q=", search_terms.join("+"), "&typefilter=strain");
    let search_resp = reqwest::get(&search_url).unwrap();
    assert!(search_resp.status().is_success());

    let doc = Document::from_read(search_resp).unwrap();
    let mut names = Vec::new();
    let mut num_reviews = Vec::new();
    let mut ratings = Vec::new();
    doc.find(Name("li").descendant(Class("padding-rowItem")).descendant(Class("copy--bold"))).for_each(|item| {
        let name = item.text().trim().to_lowercase();
        names.push(name);
    });
    doc.find(Name("li").descendant(Class("padding-rowItem")).descendant(Class("color--light"))).for_each(|item| {
        let match_chars: &[_] = &['(', ')', ' '];
        let num_revs = item.text().trim_matches(match_chars).split_whitespace().next().unwrap().to_string();
        num_reviews.push(num_revs);
    });
    doc.find(Name("li").descendant(Class("padding-rowItem")).descendant(Name("img"))).for_each(|item| {
        let rating = item.attr("src").unwrap().split('/').nth(2).unwrap();
        ratings.push(rating);
    });

    let mut review_summaries = Vec::new();
    for review in names.iter().zip(num_reviews.iter().zip(ratings.iter())) {
        let mut contains_terms = true;
        search_terms.iter().for_each(|term| {
            contains_terms &= review.0.contains(term);
        });
        if contains_terms {
            review_summaries.push(ReviewsSummary {
                source: "Leafly".to_string(),
                url: String::new(),
                strain: review.0.to_string(),
                rating: (review.1).1.parse::<f64>().unwrap(),
                ratings: (review.1).0.parse::<u32>().unwrap(),
            });
        }
    }
    review_summaries
}

fn main() {
    rocket::ignite().mount("/", routes![index]).launch();
}