use rustler::types::map::MapIterator;
use rustler::{Encoder, Env, Error, NifResult, Term};
use serde_json::{Map, Number, Value};

use crate::atoms;

include!("generated_term_encoders.rs");

pub(crate) struct EncodedLoc {
    start: usize,
    end: usize,
    start_line: usize,
    start_column: usize,
    end_line: usize,
    end_column: usize,
}

impl From<&vize_atelier_sfc::BlockLocation> for EncodedLoc {
    fn from(loc: &vize_atelier_sfc::BlockLocation) -> Self {
        Self {
            start: loc.start,
            end: loc.end,
            start_line: loc.start_line,
            start_column: loc.start_column,
            end_line: loc.end_line,
            end_column: loc.end_column,
        }
    }
}

pub(crate) fn loc_to_term<'a>(env: Env<'a>, loc: &vize_atelier_sfc::BlockLocation) -> Term<'a> {
    EncodedLoc::from(loc).encode(env)
}

pub(crate) fn attrs_to_term<'a>(
    env: Env<'a>,
    attrs: &vize_carton::FxHashMap<std::borrow::Cow<'_, str>, std::borrow::Cow<'_, str>>,
) -> Term<'a> {
    let keys: Vec<Term<'a>> = attrs.keys().map(|k| k.as_ref().encode(env)).collect();
    let vals: Vec<Term<'a>> = attrs.values().map(|v| v.as_ref().encode(env)).collect();
    if keys.is_empty() {
        Term::map_new(env)
    } else {
        Term::map_from_arrays(env, &keys, &vals).unwrap()
    }
}

struct EncodedSfcError<'a> {
    message: &'a str,
    code: Option<&'a str>,
}

impl<'a> From<&'a vize_atelier_sfc::SfcError> for EncodedSfcError<'a> {
    fn from(err: &'a vize_atelier_sfc::SfcError) -> Self {
        Self {
            message: err.message.as_str(),
            code: err.code.as_deref(),
        }
    }
}

pub(crate) struct EncodedLintDiagnostic<'a> {
    pub(crate) message: &'a str,
    pub(crate) name: &'a str,
}

pub(crate) fn nil_term<'a>(env: Env<'a>) -> Term<'a> {
    rustler::types::atom::nil().encode(env)
}

pub(crate) fn ok_term<'a, T: Encoder>(env: Env<'a>, payload: T) -> Term<'a> {
    (atoms::ok(), payload).encode(env)
}

pub(crate) fn error_term<'a, T: Encoder>(env: Env<'a>, payload: T) -> Term<'a> {
    (atoms::error(), payload).encode(env)
}

pub(crate) fn encode_json_value<'a>(env: Env<'a>, value: &Value) -> Term<'a> {
    match value {
        Value::Null => nil_term(env),
        Value::Bool(value) => value.encode(env),
        Value::Number(number) => {
            if let Some(value) = number.as_i64() {
                value.encode(env)
            } else if let Some(value) = number.as_u64() {
                value.encode(env)
            } else if let Some(value) = number.as_f64() {
                value.encode(env)
            } else {
                nil_term(env)
            }
        }
        Value::String(value) => value.encode(env),
        Value::Array(items) => {
            let terms: Vec<Term<'a>> = items
                .iter()
                .map(|item| encode_json_value(env, item))
                .collect();
            terms.encode(env)
        }
        Value::Object(map) => {
            let keys: Vec<Term<'a>> = map.keys().map(|key| key.encode(env)).collect();
            let values: Vec<Term<'a>> = map
                .values()
                .map(|value| encode_json_value(env, value))
                .collect();

            if keys.is_empty() {
                Term::map_new(env)
            } else {
                Term::map_from_arrays(env, &keys, &values).unwrap()
            }
        }
    }
}

pub(crate) fn decode_json_value(term: Term<'_>) -> NifResult<Value> {
    if term.is_empty_list() {
        return Ok(Value::Array(vec![]));
    }

    if term.is_list() {
        let items = term.decode::<Vec<Term>>()?;
        return items
            .into_iter()
            .map(decode_json_value)
            .collect::<NifResult<Vec<_>>>()
            .map(Value::Array);
    }

    if term.is_map() {
        let mut map = Map::new();
        let iterator = MapIterator::new(term).ok_or(Error::BadArg)?;

        for (key, value) in iterator {
            map.insert(
                key.decode::<std::string::String>()?,
                decode_json_value(value)?,
            );
        }

        return Ok(Value::Object(map));
    }

    if let Ok(value) = term.decode::<bool>() {
        return Ok(Value::Bool(value));
    }

    if let Ok(value) = term.decode::<i64>() {
        return Ok(Value::Number(value.into()));
    }

    if let Ok(value) = term.decode::<u64>() {
        return Ok(Value::Number(value.into()));
    }

    if let Ok(value) = term.decode::<f64>() {
        if let Some(number) = Number::from_f64(value) {
            return Ok(Value::Number(number));
        }
    }

    if let Ok(value) = term.decode::<std::string::String>() {
        return Ok(Value::String(value));
    }

    if term == nil_term(term.get_env()) {
        return Ok(Value::Null);
    }

    Err(Error::BadArg)
}

struct EncodedTemplateBlock<'a>(&'a vize_atelier_sfc::SfcTemplateBlock<'a>);

struct EncodedScriptBlock<'a>(&'a vize_atelier_sfc::SfcScriptBlock<'a>);

struct EncodedStyleBlock<'a>(&'a vize_atelier_sfc::SfcStyleBlock<'a>);

struct EncodedCustomBlock<'a>(&'a vize_atelier_sfc::SfcCustomBlock<'a>);

pub(crate) struct EncodedParseSfcResult<'a> {
    pub(crate) descriptor: &'a vize_atelier_sfc::SfcDescriptor<'a>,
}

pub(crate) struct EncodedCompileSfcResult<'a> {
    pub(crate) result: &'a vize_atelier_sfc::SfcCompileResult,
    pub(crate) code_override: Option<&'a str>,
    pub(crate) template_hash: Option<vize_carton::CompactString>,
    pub(crate) style_hash: Option<vize_carton::CompactString>,
    pub(crate) script_hash: Option<vize_carton::CompactString>,
}

struct EncodedMacroArtifact<'a>(&'a vize_atelier_sfc::SfcMacroArtifact);

pub(crate) struct EncodedTemplateCompileResult<'a> {
    pub(crate) code: &'a str,
    pub(crate) preamble: &'a str,
    pub(crate) helpers: Vec<&'a str>,
}

pub(crate) struct EncodedSsrCompileResult<'a> {
    pub(crate) code: &'a str,
    pub(crate) preamble: &'a str,
}

fn encode_css_exports<'a>(
    env: Env<'a>,
    exports: Option<
        &vize_carton::FxHashMap<vize_carton::CompactString, vize_atelier_sfc::css::CssModuleExport>,
    >,
) -> Term<'a> {
    match exports {
        Some(exports) if exports.is_empty() => nil_term(env),
        Some(exports) => {
            let keys: Vec<Term<'a>> = exports.keys().map(|key| key.as_str().encode(env)).collect();
            let values: Vec<Term<'a>> = exports
                .values()
                .map(|value| value.name.as_str().encode(env))
                .collect();
            Term::map_from_arrays(env, &keys, &values).unwrap()
        }
        None => nil_term(env),
    }
}

pub(crate) struct EncodedCssAstResult<'a> {
    pub(crate) result: &'a vize_atelier_sfc::CssAstResult,
}

pub(crate) struct EncodedCssCompileResult<'a> {
    pub(crate) result: &'a vize_atelier_sfc::CssCompileResult,
}

pub(crate) struct EncodedBundleCssResult<'a> {
    pub(crate) result: &'a vize_atelier_sfc::CssCompileResult,
}
