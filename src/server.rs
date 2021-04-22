use std::collections::HashMap;

use log::{debug, error};
use serde::Deserialize;
use tiny_http::Response;

use crate::{
    build_matcher, dictionary::Dictionary, indices_to_json, load_dicts_from_paths_and_subpaths,
    matcher::WordMatcher, morpher::Morpher, morpher::NoMorpher, search_in_dicts, settings::Opt,
};
#[derive(Deserialize)]
struct RequestBody {
    word: String,
    group: Option<String>,
}

pub fn serve_http(opt: &Opt) {
    let server = tiny_http::Server::http("127.0.0.1:51881").unwrap();
    let default_comp = build_matcher(&opt.search_algorithm, opt.search_depth);

    let mut all_dicts: HashMap<String, Dictionary> = HashMap::new();
    let mut groups: HashMap<String, (Vec<String>, Box<dyn WordMatcher + Sync>, Box<dyn Morpher>)> =
        HashMap::new();
    for g in &opt.groups {
        // load_dicts_from_paths_and_subpaths(&g.1.paths)
        //     .drain(..)
        //     .for_each(|d| {dicts.entry(d.bookname.clone()).or_insert(d); ()});
        //A dictionary that repeated in multiple groups is still loaded. Purpose of the global all_dicts is to save memory.
        let mut group_d = load_dicts_from_paths_and_subpaths(&g.1.paths);
        let mut dict_keys = Vec::new();
        for d in 0..group_d.len() {
            //Load and insert groups dictionaries to global dictionary hashmap.
            all_dicts
                .entry(group_d[d].bookname.clone())
                .or_insert(group_d.remove(d));
            dict_keys.push(group_d[d].bookname.clone());
        }
        let matcher: Box<dyn WordMatcher + Sync> =
            build_matcher(&g.1.matcher_type, g.1.matcher_depth);
        let morpher = Box::new(NoMorpher {});
        groups.insert(g.0.clone(), (dict_keys, matcher, morpher));
    }

    loop {
        let mut request = match server.recv() {
            Ok(rq) => rq,
            Err(er) => {
                println!("Error in incoming request: {}", er);
                continue;
            }
        };
        debug!("Request came from {}", &request.remote_addr());
        let req_body: RequestBody = match serde_json::from_reader(request.as_reader()) {
            Ok(n) => n,
            Err(e) => {
                request.respond(Response::empty(404)).unwrap();
                error!("Error reading request {}.", e);
                continue;
            }
        };

        let indices_to_list = if let Some(group) = req_body.group {
            let group = groups.get(&group).unwrap();
            search_in_dicts(
                &mut group.0.iter().map(|key| all_dicts.get(key).unwrap()),
                group.1.as_ref(),
                &req_body.word,
            )
        } else {
            search_in_dicts(
                &mut all_dicts.values(),
                default_comp.as_ref(),
                &req_body.word,
            )
        };

        request
            .respond(Response::from_string(indices_to_json(&indices_to_list)))
            .unwrap();
    }
}
