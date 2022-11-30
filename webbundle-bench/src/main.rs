use std::path::{Path, PathBuf};

use anyhow::Result;
use askama::Template;
use clap::Parser;
use webbundle::Bundle;

#[derive(Parser, Debug)]
struct Cli {
    /// The output directory
    #[arg(short = 'o', long, default_value = "out")]
    out: String,
    /// The module tree depth
    #[arg(short = 'd', long, default_value = "4")]
    depth: u32,
    /// The module tree width at each level
    #[arg(short = 'b', long, default_value = "4")]
    branches: u32,
    /// [Experimental] Produce two WebBundle for cache-aware WebBundles static test
    #[arg(long)]
    split: bool,
}

struct Module {
    children: Vec<Module>,
    // e.g. "a2_a1_a3"
    name: String,
    // e.g. "a1"
    short_name: String,
    // e.g. "a2/a1"
    dir: Option<PathBuf>,
}

impl Module {
    fn new(name: String, short_name: String, dir: Option<PathBuf>) -> Module {
        Module {
            name,
            short_name,
            dir,
            children: vec![],
        }
    }

    fn expand_recurse(&mut self, depth: u32, option: &Cli) {
        if depth == option.depth {
            return;
        }
        let dir = match &self.dir {
            Some(dir) => dir.join(&self.short_name),
            None => PathBuf::from(&self.short_name),
        };
        for index in 0..option.branches {
            let short_name = format!("a{index}");
            let name = format!("{}_{short_name}", self.name);
            let mut module = Module::new(name, short_name, Some(dir.clone()));
            module.expand_recurse(depth + 1, option);
            self.children.push(module);
        }
    }

    fn filename(&self) -> String {
        format!("{}.mjs", self.name)
    }

    fn full_path(&self) -> String {
        match &self.dir {
            Some(dir) => dir.join(self.filename()).display().to_string(),
            None => self.filename(),
        }
    }

    fn relative_path_from_parent(&self) -> String {
        match &self.dir {
            Some(dir) => Path::new(dir.file_name().unwrap())
                .join(self.filename())
                .display()
                .to_string(),
            None => self.filename(),
        }
    }

    fn export_function_name(&self) -> String {
        format!("f_{}", self.name)
    }

    fn function_definition(&self) -> String {
        let mut ops = self
            .children
            .iter()
            .map(|child| format!("{}()", child.export_function_name()))
            .collect::<Vec<_>>();
        ops.push("a".to_string());
        let res = ops.join(" + ");
        format!(
            r#"export function {}() {{
    let a = 1;
    return {res};
}}
"#,
            self.export_function_name()
        )
    }

    fn import_me(&self) -> String {
        format!(
            r#"import {{ {} }} from "./{}""#,
            self.export_function_name(),
            self.relative_path_from_parent()
        )
    }

    fn export(&self, mut builder: webbundle::Builder, option: &Cli) -> Result<webbundle::Builder> {
        match &self.dir {
            Some(dir) => log::debug!("{}", dir.join(self.filename()).display()),
            None => log::debug!("{}", self.filename()),
        };
        let t = ModuleTemplate {
            imports: self
                .children
                .iter()
                .map(|child| child.import_me())
                .collect(),
            function_definition: self.function_definition(),
        };

        let output_dir = match &self.dir {
            Some(dir) => PathBuf::from(&option.out).join(dir),
            None => PathBuf::from(&option.out),
        };

        std::fs::create_dir_all(&output_dir)?;

        let file = PathBuf::from(&option.out).join(self.full_path());
        std::fs::write(file, t.render().unwrap())?;

        builder = builder.exchange((self.full_path(), t.render().unwrap().into_bytes()).into());

        for child in &self.children {
            builder = child.export(builder, option)?;
        }
        Ok(builder)
    }
}

trait Resources {
    fn resources(&self) -> Vec<String>;
}

impl Resources for Bundle {
    fn resources(&self) -> Vec<String> {
        self.exchanges()
            .iter()
            .map(|e| format!(r#""{}""#, e.request.url()))
            .collect::<Vec<_>>()
    }
}

struct Benchmark {
    start_module: Module,
}

const CACHE_HIT: [usize; 11] = [0, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100];

impl Benchmark {
    fn new(option: &Cli) -> Benchmark {
        let mut start_module = Module::new("a0".to_string(), "a0".to_string(), None);
        start_module.expand_recurse(0, option);
        Benchmark { start_module }
    }

    fn build(&self, option: &Cli) -> Result<()> {
        let bundle = self.build_modules(option)?;
        self.build_html(option)?;

        // For cache-aware Web Bundle ad-hoc tests.
        if option.split {
            for cache_hit in CACHE_HIT {
                let (bundle0, bundle1) =
                    self.build_cache_aware_bundle(option, &bundle, cache_hit)?;
                self.build_cache_aware_bundle_html(option, &bundle0, &bundle1, cache_hit)?;
            }
        }
        Ok(())
    }

    fn build_modules(&self, option: &Cli) -> Result<Bundle> {
        // Build modules
        let builder = Bundle::builder().version(webbundle::Version::VersionB2);
        let builder = self.start_module.export(builder, option)?;

        // Build webbundle
        let bundle = builder.build()?;
        println!("Build {} modules", bundle.exchanges().len());
        std::fs::create_dir_all(&option.out)?;
        let f = std::fs::File::create(PathBuf::from(&option.out).join("webbundle.wbn"))?;
        bundle.write_to(f)?;

        Ok(bundle)
    }

    fn build_cache_aware_bundle(
        &self,
        option: &Cli,
        bundle: &Bundle,
        cache_hit: usize,
    ) -> Result<(Bundle, Bundle)> {
        let mut builder0 = Bundle::builder().version(webbundle::Version::VersionB2);
        let mut builder1 = Bundle::builder().version(webbundle::Version::VersionB2);
        let len = bundle.exchanges().len();
        for (i, exchange) in bundle.exchanges().iter().enumerate() {
            if i * 100 < len * cache_hit {
                builder0 = builder0.exchange(exchange.clone());
            } else {
                builder1 = builder1.exchange(exchange.clone());
            }
        }

        let bundle0 = builder0.build()?;
        let bundle1 = builder1.build()?;

        let f = std::fs::File::create(
            PathBuf::from(&option.out).join(format!("webbundle-cache-aware-{cache_hit}.wbn")),
        )?;
        bundle0.write_to(f)?;

        let dir = PathBuf::from(&option.out).join("cache-aware-2nd");
        std::fs::create_dir_all(&dir)?;
        let f = std::fs::File::create(dir.join(format!("webbundle-cache-aware-{cache_hit}.wbn")))?;
        bundle1.write_to(f)?;

        Ok((bundle0, bundle1))
    }

    fn build_html(&self, option: &Cli) -> Result<()> {
        self.build_unbundled_html(option)?;
        self.build_webbundle_html(option)?;
        self.build_index_html(option)
    }

    fn build_unbundled_html(&self, option: &Cli) -> Result<()> {
        let t = BenchmarkTemplate {
            headers: "".to_string(),
            info: format!("option: {option:#?}"),
            modules: vec![],
            start_module: self.start_module.full_path(),
            start_func: self.start_module.export_function_name(),
            next_links: vec![],
        };

        std::fs::create_dir_all(&option.out)?;
        let file = PathBuf::from(&option.out).join("unbundled.html");
        std::fs::write(file, t.render().unwrap())?;
        Ok(())
    }

    fn build_webbundle_html(&self, option: &Cli) -> Result<()> {
        let t = BenchmarkTemplate {
            headers: r#"<script type="webbundle"> { "source": "webbundle.wbn", "scopes": ["."] } </script>"#.to_string(),
            info: format!("option: {option:#?}"),
            modules: vec![],
            start_module: self.start_module.full_path(),
            start_func: self.start_module.export_function_name(),
            next_links: vec![],
        };

        std::fs::create_dir_all(&option.out)?;
        let file = PathBuf::from(&option.out).join("webbundle.html");
        std::fs::write(file, t.render().unwrap())?;
        Ok(())
    }

    fn build_cache_aware_bundle_html(
        &self,
        option: &Cli,
        bundle0: &Bundle,
        bundle1: &Bundle,
        cache_hit: usize,
    ) -> Result<()> {
        let bundle_source_name = format!("webbundle-cache-aware-{cache_hit}.wbn");

        // Html for 1st visit.
        {
            let resources = bundle0.resources().join(", ");

            let t = BenchmarkTemplate {
                headers: format!(
                    r#"<script type="bundlepreload"> {{ "source": "{bundle_source_name}", "resources": [ {resources} ] }} </script>"#
                ),
                info: format!("option: {option:#?}"),
                modules: vec![],
                start_module: self.start_module.full_path(),
                start_func: self.start_module.export_function_name(),
                next_links: vec![format!("webbundle-cache-aware-{cache_hit}-2nd.html")],
            };

            std::fs::create_dir_all(&option.out)?;
            let file = PathBuf::from(&option.out)
                .join(format!("webbundle-cache-aware-{cache_hit}-1st.html"));
            std::fs::write(file, t.render().unwrap())?;
        }

        // Html for 2nd visit.
        {
            let resources = {
                let mut resources = bundle0.resources();
                resources.append(&mut bundle1.resources());
                resources.join(", ")
            };

            let t = BenchmarkTemplate {
                headers: format!(
                    r#"<script type="bundlepreload"> {{ "source": "{bundle_source_name}", "resources": [ {resources} ] }} </script>"#
                ),
                info: format!("option: {option:#?}"),
                modules: vec![],
                start_module: self.start_module.full_path(),
                start_func: self.start_module.export_function_name(),
                next_links: vec![],
            };

            std::fs::create_dir_all(&option.out)?;
            let file = PathBuf::from(&option.out)
                .join(format!("webbundle-cache-aware-{cache_hit}-2nd.html"));
            std::fs::write(file, t.render().unwrap())?;
        }
        Ok(())
    }

    fn build_index_html(&self, option: &Cli) -> Result<()> {
        let mut benchmarks = vec!["unbundled".to_string(), "webbundle".to_string()];
        if option.split {
            for cache_hit in CACHE_HIT {
                benchmarks.push(format!("webbundle-cache-aware-{cache_hit}-1st"));
            }
        }
        let t = IndexTemplate {
            info: format!("option: {option:#?}"),
            benchmarks,
        };

        std::fs::create_dir_all(&option.out)?;
        let file = PathBuf::from(&option.out).join("index.html");
        std::fs::write(file, t.render().unwrap())?;
        Ok(())
    }
}

#[derive(Template)]
#[template(path = "module.html")]
struct ModuleTemplate {
    imports: Vec<String>,
    function_definition: String,
}

#[derive(Template)]
#[template(path = "benchmark.html")]
struct BenchmarkTemplate {
    headers: String,
    info: String,
    modules: Vec<String>,
    start_module: String,
    start_func: String,
    next_links: Vec<String>,
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    info: String,
    benchmarks: Vec<String>,
}

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();
    let benchmark = Benchmark::new(&cli);
    benchmark.build(&cli)
}
