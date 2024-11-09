use std::{
    path::{Path, PathBuf},
    sync::{LazyLock, OnceLock},
};

use actix_web::{get, http::StatusCode, web, App, HttpResponse, HttpServer};
use anyhow::{Context, Result};
use clap::{CommandFactory, FromArgMatches};
use labelme_rs::{load_label_colors, LabelColorsHex, LabelMeDataWImage};
use lmrs::cli::{BrowseCmdArgs as CmdArgs, BrowseServerConfig, SvgConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
struct AppState {
    svg: SvgConfig,
    dir: PathBuf,
    label_colors: LabelColorsHex,
    templates: tera::Tera,
}

static PARENT_DIR: OnceLock<PathBuf> = OnceLock::new();

static ID_LIST: LazyLock<Vec<String>> = LazyLock::new(|| {
    let dir = PARENT_DIR.get().unwrap(); // PARENT_DIR is initialized in actix_main
    let entries = std::fs::read_dir(dir)
        .with_context(|| format!("Failed to read directory: {:?}", dir))
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
    v_id_list
});

fn _get_svg(app_state: &web::Data<AppState>, id: &String) -> Result<String> {
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
    let svg = _get_svg(&app_state, &id).with_context(|| format!("Failed to get svg for {}", id));
    match svg {
        Ok(svg) => HttpResponse::build(StatusCode::OK)
            .content_type("image/svg+xml")
            .body(svg),
        Err(e) => HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR)
            .content_type("text/plain")
            .body(
                e.chain()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("\n")
                    .to_string(),
            ),
    }
}

fn _browse_id(app_state: web::Data<AppState>, id: &String, no_nav: bool) -> Result<String> {
    let svg = _get_svg(&app_state, id)?;
    let mut context = tera::Context::new();
    context.insert("title", &format!("{} - lmrs browse", id));
    context.insert("svg", &svg);
    let pos = (*ID_LIST).binary_search(id);
    if !no_nav {
        if let Ok(pos) = pos {
            if pos > 0 {
                context.insert("prev_id", &(*ID_LIST)[pos - 1]);
            }
            if pos < (*ID_LIST).len() - 1 {
                context.insert("next_id", &(*ID_LIST)[pos + 1]);
            }
        }
    }

    let html = app_state
        .templates
        .render("browse_id.jinja", &context)
        .context("Failed to render template")?;
    Ok(html)
}

#[derive(Deserialize)]
struct BrowseIdQuery {
    no_nav: Option<bool>,
}

#[get("/browse/{id}")]
async fn browse_id(
    query: web::Query<BrowseIdQuery>,
    app_state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let id = path.into_inner();
    let no_nav: bool = query.no_nav.unwrap_or_default();
    let html = _browse_id(app_state, &id, no_nav)
        .with_context(|| format!("Failed to get html for {}", id));
    match html {
        Ok(html) => HttpResponse::build(StatusCode::OK)
            .content_type("text/html")
            .body(html),
        Err(e) => HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR)
            .content_type("text/plain")
            .body(
                e.chain()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("\n")
                    .to_string(),
            ),
    }
}

#[get("/")]
async fn index(_app_state: web::Data<AppState>) -> HttpResponse {
    let id_list = &*ID_LIST;

    let list = id_list
        .iter()
        .map(|id| {
            format!(
                "<head><title>lmrs browse</title></head><li><a href=\"/browse/{0}\">{0}</a></li>",
                id
            )
        })
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
    default_url_path: String,
    args: CmdArgs,
    app_state: AppState,
) -> std::io::Result<()> {
    let http_server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .service(index)
            .service(browse_id)
            .service(get_svg)
    })
    .workers(1)
    .bind((config.server.address, config.server.port))?;
    let addr = *http_server.addrs().first().unwrap();
    let server = http_server.run();

    let default_url = format!("http://{}:{}{}", addr.ip(), addr.port(), default_url_path);

    println!("Open {}", default_url);

    if args.open {
        let result = open::that(default_url);
        if let Err(e) = result {
            error!("Failed to open browser: {}", e);
        }
    }

    server.await
}

/// Config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Server address
    pub server: BrowseServerConfig,
    /// SVG
    pub svg: SvgConfig,
}

impl Default for Config {
    fn default() -> Self {
        let svg = SvgConfig {
            resize: Some("512x512".to_string()),
            ..Default::default()
        };
        Self {
            svg,
            server: BrowseServerConfig::default(),
        }
    }
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

    // Initialize config from file
    let config: Config = if let Some(path) = args.base_config.as_ref() {
        toml::from_str(&std::fs::read_to_string(path)?)?
    } else {
        load_config_from_config_dir()
            .or_else(load_config_next_to_executable)
            .unwrap_or_default()
    };

    // Update config from arguments
    args.server = config.server.clone();
    args.svg = config.svg.clone();

    // Cut-off arguments before `browse`
    let command_args = std::env::args()
        .skip_while(|arg| arg != "browse")
        .collect::<Vec<_>>();

    let matches = <CmdArgs as CommandFactory>::command().get_matches_from(command_args);

    args.update_from_arg_matches(&matches)?;

    let config = Config {
        server: args.server.clone(),
        svg: args.svg.clone(),
    };

    if !args.input.exists() {
        panic!("Input file does not exist: {:?}", args.input);
    }

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
            format!("/browse/{}?no_nav=true", stem)
        } else {
            panic!("Invalid input file: {:?}", args.input);
        }
    } else {
        "".to_string()
    };

    let templates = get_templates();

    PARENT_DIR.get_or_init(|| dir.clone());

    let app_state = AppState {
        svg: config.svg.clone(),
        dir,
        label_colors,
        templates,
    };

    actix_main(config, default_url, args, app_state).context("Failed to start actix server")?;
    Ok(())
}

fn get_templates() -> tera::Tera {
    let mut templates = tera::Tera::default();
    templates.autoescape_on(vec![]);
    templates
        .add_raw_templates(vec![(
            "browse_id.jinja",
            include_str!("templates/browse_id.jinja"),
        )])
        .unwrap();
    templates
}

#[cfg(test)]
mod tests {
    use actix_web::{http::header::ContentType, test, App};

    use super::*;

    fn init_app_state() -> AppState {
        let config = Config::default();
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/data");
        PARENT_DIR.get_or_init(|| dir.clone());
        let templates = get_templates();

        AppState {
            svg: config.svg.clone(),
            dir,
            label_colors: LabelColorsHex::new(),
            templates,
        }
    }

    #[actix_web::test]
    async fn test_index_get() {
        let app_state = init_app_state();
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
    async fn test_gets() {
        let app_state = init_app_state();
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(app_state.clone()))
                .service(get_svg)
                .service(browse_id),
        )
        .await;
        let req = test::TestRequest::get().uri("/svg/Mandrill").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let req = test::TestRequest::get()
            .uri("/browse/Mandrill")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }
}
