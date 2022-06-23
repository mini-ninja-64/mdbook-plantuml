use clap::{Arg, Command};
use mdbook::book::Book;
use mdbook::errors::Error as MDBookError;
use mdbook::preprocess::{CmdPreprocessor, Preprocessor, PreprocessorContext};
use mdbook_plantuml::PlantUMLPreprocessor;
use mdbook_plantuml::plantumlconfig::{get_plantuml_config, PlantUMLConfig};
use std::io;
use std::process;

pub fn make_app() -> Command<'static> {
    const VERSION: &str = env!("CARGO_PKG_VERSION");

    Command::new("mdBook PlantUML preprocessor")
        .version(VERSION)
        .author("Sytse Reitsma")
        .about("An mdbook preprocessor which renders PlantUML code blocks to SVG diagrams")
        .subcommand(
            Command::new("supports")
                .arg(Arg::new("renderer").required(true))
                .about("Check whether a renderer is supported by this preprocessor"),
        )
}

fn main() {
    let matches = make_app().get_matches();

    let preprocessor = PlantUMLPreprocessor;

    if let Some(sub_args) = matches.subcommand_matches("supports") {
        // It is safe to unwrap as it is confirmed to be there by clap validation
        handle_supports(&preprocessor, sub_args.value_of("renderer").unwrap());
    } else {
        let preprocessor_result = CmdPreprocessor::parse_input(io::stdin());

    if let Ok((preprocessor_context, book)) = preprocessor_result {
        let cfg = get_plantuml_config(&preprocessor_context);

        handle_setup(&preprocessor, &preprocessor_context, &cfg);
        handle_preprocessing(&preprocessor, &preprocessor_context, &book);
    } else {
        eprintln!("{}", preprocessor_result.unwrap_err());
        process::exit(1);

    }
    }
}


fn handle_setup(preprocessor: &dyn Preprocessor, preprocessor_context: &PreprocessorContext,cfg: &PlantUMLConfig) {
    if cfg.enable_logging {
        if let Err(e) = setup_logging(&cfg) {
                    eprintln!("{}", e);
                    process::exit(2);
                }
    }

    if preprocessor_context.mdbook_version != mdbook::MDBOOK_VERSION {
        // We should probably use the `semver` crate to check compatibility
        // here...
        log::error!("Warning: The {} plugin was built against version {} of mdbook, but we're being called from version {}", preprocessor.name(), mdbook::MDBOOK_VERSION, preprocessor_context.mdbook_version);
    }
}

fn handle_preprocessing(preprocessor: &dyn Preprocessor, preprocessor_context: &PreprocessorContext, book: &Book) -> Result<(), MDBookError>{
    let processed_book = preprocessor.run(&preprocessor_context, book.to_owned())?;
    serde_json::to_writer(io::stdout(), &processed_book)?;
    Ok(())
}

fn handle_supports(pre: &dyn Preprocessor, renderer: &str) -> ! {
    let supported = pre.supports_renderer(renderer);

    // Signal whether the renderer is supported by exiting with 1 or 0.
    if supported {
        process::exit(0);
    } else {
        process::exit(1);
    }
}

fn setup_logging(config: &PlantUMLConfig) -> Result<(), MDBookError> {
    use log::LevelFilter;
    use log4rs::append::file::FileAppender;
    use log4rs::config::{Appender, Config, Root};
    use log4rs::encode::pattern::PatternEncoder;
    
    if !config.enable_logging { 
        return Ok(()); 
    }

    if let Some(config_file) = config.logging_config.as_ref() {
        log4rs::init_file(config_file, Default::default())?;
    } else {
        // use default logging configuration
        let logfile_appender = FileAppender::builder()
            .encoder(Box::new(PatternEncoder::new("{l} - {m}\n")))
            .build("output.log")?;

        let config = Config::builder()
            .appender(Appender::builder().build("logfile", Box::new(logfile_appender)))
            .build(
                Root::builder()
                    .appender("logfile")
                    .build(LevelFilter::Debug),
            )?;
        log4rs::init_config(config)?;
    }
    Ok(())
}
