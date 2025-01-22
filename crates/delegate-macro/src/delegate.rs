use syn::{
    parse::{Parse, ParseStream},
    Attribute,
};

/// Parsed input for `delegator!` macro.
#[derive(Clone, Debug)]
pub struct DelegatorInput {
    /// All transaction variants
    pub variants: Vec<Attribute>,
}

impl Parse for DelegatorInput {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        // // Ignore outer attributes when peeking.
        // let fork = input.fork();
        // let _fork_outer = Attribute::parse_outer(&fork)?;
        //
        // if fork.peek(LitStr) || (fork.peek(Ident) && fork.peek2(Token![,]) && fork.peek3(LitStr))
        // {     Self::parse_abigen(attrs, input)
        // } else {
        //     input.parse().map(|kind| Self { attrs, path: None, kind })
        // }
        todo!()
    }
}
