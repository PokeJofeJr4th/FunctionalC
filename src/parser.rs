lalrpop_util::lalrpop_mod! {
    #[allow(clippy::pedantic, clippy::nursery)]
    pub grammar,
    "/parser/grammar.rs"
}

pub mod syntax;
