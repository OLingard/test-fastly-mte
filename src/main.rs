use std::collections::HashMap;

use anyhow::Context;
use fastly::http::header::HOST;
// use fastly::experimental::BackendExt;
use fastly::http::{StatusCode};
use fastly::{Backend, ConfigStore, Error, KVStore, Request, Response};
use logger::logger::JsonLogger;

mod logger;

#[fastly::main]
fn main(mut req: Request) -> Result<Response, Error> {
    
    match init_logger() {
        Err(e) => println!("{}", e),
        _ => {}
    };

    let forwarded_host = match req.get_header("x-forwarded-host") {
        None => match req.get_header(HOST) {
            None => {
                log::error!("No Host or Forwarded Host Header Found");
                return Ok(Response::from_status(StatusCode::BAD_REQUEST));
            }
            Some(s) => s.to_str().unwrap(),
        },
        Some(s) => s.to_str().unwrap(),
    };

    log::info!("Forwarded Host with host fallback: {}", forwarded_host);
    // for header_name in req.get_header_names_str() {
    //     log::info!("{}: {}", header_name, req.get_header_str(header_name).get_or_insert("foo"));
    // }

    let mapping_store = ConfigStore::open("mapper_config_link");

    let routes_id = match mapping_store.try_get(&forwarded_host) {
        Err(e) => {
            log::error!("Config store get error: {}", e);
            return Ok(Response::from_status(StatusCode::INTERNAL_SERVER_ERROR));
        }
        Ok(None) => {
            log::warn!("Host routes not found in Config store");
            return Ok(Response::from_status(StatusCode::OK));
        }
        Ok(Some(s)) => s,
    };

    log::info!("Route ID: {}", routes_id);

    let routes_store = match KVStore::open("routes_kv_link") {
        Err(e) => {
            log::error!("KV store error: {}", e);
            return Ok(Response::from_status(StatusCode::INTERNAL_SERVER_ERROR));
        }
        Ok(None) => {
            log::error!("No KV store found");
            return Ok(Response::from_status(StatusCode::INTERNAL_SERVER_ERROR));
        }
        Ok(Some(s)) => s,
    };

    let routes_json = match routes_store.lookup_str(&routes_id) {
        Err(e) => {
            log::error!("KV store error: {}", e);
            return Ok(Response::from_status(StatusCode::INTERNAL_SERVER_ERROR));
        }
        Ok(None) => {
            log::warn!("Routes ID {} not found in KV", routes_id);
            return Ok(Response::from_status(StatusCode::OK));
        }
        Ok(Some(s)) => s,
    };

    let req_full_path = match req.get_query_str() {
        None => req.get_path().to_string(),
        Some(query) => {
            format!("{}?{}", req.get_path(), query)
        }
    };

    log::info!("Full Path: {}", req_full_path);

    let routes: HashMap<String, String> = match serde_json::from_str(&routes_json) {
        Ok(j) => j,
        Err(e) => {
            log::error!("Routes parsing error: {}", e);
            return Ok(Response::from_status(StatusCode::INTERNAL_SERVER_ERROR));
        }
    };

    for route in routes {
        if req_full_path.starts_with(&route.0) {
            let new_host = route.1;
            log::info!("New Host: {}", new_host);
            let mut new_location = req.get_url().clone();
            new_location.set_host(Some(&new_host))?;
            let new_path = crop_letters(req.get_path(), route.0.len());
            log::info!("New Path: {}", new_path);
            new_location.set_path(new_path);

            let mut backend_builder =
                Backend::builder(backend_host_name(&new_host), &new_host).override_host(&new_host);
        
            if new_location.scheme() == "https" {
                log::info!("Https Enabled");
                backend_builder = backend_builder.enable_ssl();
                backend_builder = backend_builder.override_host(&new_host);
                backend_builder = backend_builder.sni_hostname(&new_host);
            }

            req.set_url(new_location);
        
            let backend_route = match backend_builder.finish().context("Dynamic backend failed") {
                Ok(r) => r,
                Err(e) => {
                    log::error!("Routes parsing error: {}", e);
                    return Ok(Response::from_status(StatusCode::INTERNAL_SERVER_ERROR));
                }
            };
            req.set_pass(true);
            let resp = match req.send(backend_route) {
                Ok(r) => r,
                Err(e) => {
                    log::error!("Send Error: {}", e);
                    return Ok(Response::from_status(StatusCode::INTERNAL_SERVER_ERROR));
                }
            };
            log::info!("Origin Status: {}", resp.get_status().as_str());
            
            return Ok(resp);
        }
    }
    return Ok(Response::from_status(StatusCode::BAD_REQUEST));
}

fn crop_letters(s: &str, pos: usize) -> &str {
    match s.char_indices().skip(pos).next() {
        Some((pos, _)) => &s[pos..],
        None => "",
    }
}

fn backend_host_name(host: &String) -> String {
    return host.replace(|c: char| !c.is_alphanumeric(), "")
}

fn init_logger() -> Result<(), String> {
    let fastly_logger = match log_fastly::Logger::builder()
        .max_level(log::LevelFilter::Info)
        .default_endpoint("logs")
        .echo_stdout(true)
        .build()
    {
        Ok(l) => l,
        Err(e) => return Err(e.to_string()),
    };
    let trace_id = std::env::var("FASTLY_TRACE_ID").unwrap_or_else(|_| String::new());

    let json_logger = JsonLogger::new(fastly_logger, trace_id);
    log::set_max_level(log::LevelFilter::Info);

    match log::set_boxed_logger(Box::new(json_logger)) {
        Err(e) => return Err(e.to_string()),
        _ => {}
    };
    Ok(())
}