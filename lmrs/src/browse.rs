use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use actix_web::{get, http::StatusCode, web, App, HttpResponse, HttpServer};
use anyhow::{Context, Result};
use labelme_rs::{load_label_colors, LabelColorsHex, LabelMeDataWImage};
use lmrs::cli::{BrowseCmdArgs as CmdArgs, BrowseServerConfig, SvgConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
struct AppState {
    svg: SvgConfig,
    dir: PathBuf,
    id_list: Arc<Mutex<Option<Vec<String>>>>,
    label_colors: LabelColorsHex,
}

fn _get_svg(app_state: web::Data<AppState>, id: &String) -> Result<String> {
    let path = app_state.dir.join(id).with_extension("json");
    let mut data_image = LabelMeDataWImage::try_from(path.as_path())?;
    if let Some(resize) = app_state.svg.resize.as_ref() {
        let resize_param = labelme_rs::ResizeParam::try_from(resize.as_str())?;
        data_image.resize(&resize_param);
    }
    let data = data_image.data;
    let svg = data.to_svg(
        &app_state.label_colors,
        app_state.svg.radius,
        app_state.svg.line_width,
        &data_image.image,
    );
    Ok(svg.to_string())
}

#[get("/svg/{id}")]
async fn get_svg(app_state: web::Data<AppState>, path: web::Path<String>) -> HttpResponse {
    let id = path.into_inner();
    let svg = _get_svg(app_state, &id).with_context(|| format!("Failed to get svg for {}", id));
    match svg {
        Ok(svg) => HttpResponse::build(StatusCode::OK)
            .content_type("image/svg+xml")
            .body(svg),
        Err(e) => HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR)
            .content_type("text/plain")
            .body(format!("{}", e)),
    }
}

#[get("/")]
async fn index(app_state: web::Data<AppState>) -> HttpResponse {
    let mut id_list = app_state.id_list.lock().unwrap();
    if id_list.is_none() {
        let entries = std::fs::read_dir(&app_state.dir)
            .with_context(|| format!("Failed to read directory: {:?}", app_state.dir))
            .unwrap();
        let mut v_id_list = Vec::new();
        for entry in entries {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().unwrap_or_default() == "json" {
                let id = path.file_stem().unwrap().to_str().unwrap();
                v_id_list.push(id.to_string());
            }
        }
        v_id_list.sort();
        *id_list = Some(v_id_list);
    }

    let list = id_list
        .as_ref()
        .unwrap()
        .iter()
        .map(|id| format!("<li><a href=\"/svg/{0}\">{0}</a></li>", id))
        .collect::<Vec<String>>()
        .join("\n");
    let body = format!("<ul>{}</ul>", list);
    HttpResponse::build(StatusCode::OK)
        .content_type("text/html")
        .body(body)
}

#[actix_web::main]
async fn actix_main(
    config: Config,
    default_url: String,
    args: CmdArgs,
    app_state: AppState,
) -> std::io::Result<()> {
    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .service(get_svg)
            .service(index)
    })
    .bind((config.server.address, config.server.port))?
    .run();

    if args.open {
        let result = open::that(default_url);
        if let Err(e) = result {
            error!("Failed to open browser: {}", e);
        }
    }

    server.await
}

/// Config
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Server address
    pub server: BrowseServerConfig,
    /// SVG
    pub svg: SvgConfig,
}

fn load_config(config_dir: &Path) -> Result<Option<Config>> {
    let config_path = config_dir.join("lmrs_browse.toml");
    if config_path.exists() {
        debug!("config_path: {:?}", config_path);
        return Ok(toml::from_str(&std::fs::read_to_string(config_path)?)?);
    } else {
        debug!("config_path: {:?} does not exist", config_path);
    }
    Ok(None)
}

fn _load_config_from_config_dir() -> Result<Option<Config>> {
    if let Some(config_dir) = dirs::config_dir() {
        return load_config(&config_dir.join("lmrs"));
    }
    Ok(None)
}

fn load_config_from_config_dir() -> Option<Config> {
    match _load_config_from_config_dir() {
        Ok(config) => config,
        Err(e) => {
            error!("Failed to load config from config dir: {}", e);
            None
        }
    }
}

fn _load_config_next_to_executable() -> Result<Option<Config>> {
    let exe_path = std::env::current_exe()?;
    let exe_dir = exe_path.parent().unwrap();
    load_config(exe_dir)
}

fn load_config_next_to_executable() -> Option<Config> {
    match _load_config_next_to_executable() {
        Ok(config) => config,
        Err(e) => {
            error!("Failed to load config next to executable: {}", e);
            None
        }
    }
}

pub fn cmd(mut args: CmdArgs) -> Result<()> {
    if args.default {
        let config = Config::default();
        let toml = toml::to_string(&config).unwrap();
        println!("{}", toml);
        return Ok(());
    }
    let config: Config = if let Some(path) = args.config.as_ref() {
        toml::from_str(&std::fs::read_to_string(path)?)?
    } else {
        load_config_from_config_dir()
            .or_else(load_config_next_to_executable)
            .unwrap_or_default()
    };

    if args.input.extension().unwrap_or_default() == "jpg" {
        // Find adjacent json file
        let json = args.input.with_extension("json");
        debug!("json: {:?}", json);
        args.input = json;
    }

    let dir = if args.input.is_file() {
        args.input.parent().unwrap().to_path_buf()
    } else {
        args.input.clone()
    };
    let label_colors = match &config.svg.config {
        Some(config) => load_label_colors(config)?,
        None => LabelColorsHex::new(),
    };

    let default_url = if args.input.is_file() {
        if args.input.extension().unwrap_or_default() == "json" {
            let stem = args.input.file_stem().unwrap().to_str().unwrap();
            format!(
                "http://{}:{}/svg/{}",
                config.server.address, config.server.port, stem
            )
        } else {
            panic!("Invalid input file: {:?}", args.input);
        }
    } else {
        format!("http://{}:{}", config.server.address, config.server.port)
    };
    println!("Open {}", default_url);

    let app_state = AppState {
        svg: config.svg.clone(),
        dir,
        label_colors,
        id_list: Arc::new(Mutex::new(None)),
    };

    actix_main(config, default_url, args, app_state).context("Failed to start actix server")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use actix_web::{http::header::ContentType, test, App};

    use super::*;

    #[actix_web::test]
    async fn test_index_get() {
        let config = Config::default();
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/data");

        let app_state = AppState {
            svg: config.svg.clone(),
            dir,
            id_list: Arc::new(Mutex::new(None)),
            label_colors: LabelColorsHex::new(),
        };
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(app_state.clone()))
                .service(index),
        )
        .await;
        let req = test::TestRequest::default()
            .insert_header(ContentType::plaintext())
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    #[actix_web::test]
    async fn test_svg_get() {
        let config = Config::default();
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/data");

        let app_state = AppState {
            svg: config.svg.clone(),
            dir,
            id_list: Arc::new(Mutex::new(None)),
            label_colors: LabelColorsHex::new(),
        };
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(app_state.clone()))
                .service(get_svg),
        )
        .await;
        let req = test::TestRequest::get().uri("/svg/Mandrill").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }
}
