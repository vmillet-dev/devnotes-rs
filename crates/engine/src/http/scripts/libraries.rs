//! What a script can `require`: the libraries a Postman script reaches and Node's own modules,
//! bundled by `sandbox/build.mjs` into `sandbox/dist/` and compiled in. Only the script host
//! evaluates them, each on its first `require`.

macro_rules! bundled {
    ($($name:literal => $file:literal),+ $(,)?) => {
        const LIBRARIES: &[(&str, &str)] = &[
            $(($name, include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/sandbox/dist/", $file)))),+
        ];
    };
}

bundled! {
    "assert" => "assert.js",
    "buffer" => "buffer.js",
    "events" => "events.js",
    "path" => "path.js",
    "punycode" => "punycode.js",
    "querystring" => "querystring.js",
    "stream" => "stream.js",
    "string_decoder" => "string-decoder.js",
    "timers" => "timers.js",
    "url" => "url.js",
    "util" => "util.js",
    "ajv" => "ajv.js",
    "chai" => "chai.js",
    "cheerio" => "cheerio.js",
    "crypto-js" => "crypto-js.js",
    "csv-parse" => "csv-parse.js",
    "csv-parse/sync" => "csv-parse-sync.js",
    "lodash" => "lodash.js",
    "moment" => "moment.js",
    "tv4" => "tv4.js",
    "uuid" => "uuid.js",
    "xml2js" => "xml2js.js",
}

/// The module's source, `None` for a name nothing provides.
pub(crate) fn source(name: &str) -> Option<&'static str> {
    LIBRARIES
        .iter()
        .find(|(known, _)| *known == name)
        .map(|(_, source)| *source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(serde::Deserialize)]
    struct Bundled {
        name: String,
    }

    /// The build writes what it bundled: a module it added and this table forgot cannot be
    /// required, and one removed would not compile.
    #[test]
    fn every_module_the_build_wrote_is_in_the_table() {
        let manifest: Vec<Bundled> = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/sandbox/dist/manifest.json"
        )))
        .unwrap();
        assert_eq!(manifest.len(), LIBRARIES.len());
        for module in manifest {
            assert!(
                source(&module.name).is_some(),
                "{} is not compiled in",
                module.name
            );
        }
    }
}
