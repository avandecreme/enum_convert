use proc_macro2::{Delimiter, Group, Ident, Punct, Spacing, Span, TokenStream, TokenTree};

use crate::idents::{ContainerIdent, FieldIdent, ToTokenStream, VariantIdent};

pub fn ident(name: &str, span: Span) -> TokenStream {
    TokenStream::from(TokenTree::Ident(Ident::new(name, span)))
}

pub fn field_ident(pos: usize, span: Span) -> Ident {
    Ident::new(&format!("field_{pos}"), span)
}

pub fn punct(ch: char) -> TokenStream {
    TokenStream::from(TokenTree::Punct(Punct::new(ch, Spacing::Alone)))
}

pub fn punct_joint(ch: char) -> TokenStream {
    TokenStream::from(TokenTree::Punct(Punct::new(ch, Spacing::Joint)))
}

pub fn group(delimiter: Delimiter, content: TokenStream) -> TokenStream {
    TokenStream::from(TokenTree::Group(Group::new(delimiter, content)))
}

pub fn path_sep() -> TokenStream {
    let mut ts = TokenStream::new();
    ts.extend(punct_joint(':'));
    ts.extend(punct(':'));
    ts
}

pub fn fat_arrow() -> TokenStream {
    let mut ts = TokenStream::new();
    ts.extend(punct_joint('='));
    ts.extend(punct('>'));
    ts
}

pub fn impl_from(
    source_enum: &ContainerIdent,
    target_enum: &ContainerIdent,
    match_arms: TokenStream,
) -> TokenStream {
    let span = Span::call_site();
    let mut ts = TokenStream::new();

    // impl From<source_enum> for target_enum
    ts.extend(ident("impl", span));
    ts.extend(ident("From", span));
    ts.extend(punct('<'));
    ts.extend(source_enum.to_token_stream());
    ts.extend(punct('>'));
    ts.extend(ident("for", span));
    ts.extend(target_enum.to_token_stream());

    // { fn from(value: source_enum) -> Self { match value { ... } } }
    let mut body = TokenStream::new();
    body.extend(ident("fn", span));
    body.extend(ident("from", span));

    // (value: source_enum)
    let mut params = TokenStream::new();
    params.extend(ident("value", span));
    params.extend(punct(':'));
    params.extend(source_enum.to_token_stream());
    body.extend(group(Delimiter::Parenthesis, params));

    // -> Self
    body.extend(punct_joint('-'));
    body.extend(punct('>'));
    body.extend(ident("Self", span));

    // { match value { ... } }
    let mut fn_body = TokenStream::new();
    fn_body.extend(ident("match", span));
    fn_body.extend(ident("value", span));
    fn_body.extend(group(Delimiter::Brace, match_arms));
    body.extend(group(Delimiter::Brace, fn_body));

    ts.extend(group(Delimiter::Brace, body));
    ts
}

pub fn unit_to_unit_arm(
    source_enum: &ContainerIdent,
    source_variant: &VariantIdent,
    target_enum: &ContainerIdent,
    target_variant: &VariantIdent,
) -> TokenStream {
    let mut ts = TokenStream::new();

    // source_enum::source_variant
    ts.extend(source_enum.to_token_stream());
    ts.extend(path_sep());
    ts.extend(source_variant.to_token_stream());

    // =>
    ts.extend(fat_arrow());

    // target_enum::target_variant,
    ts.extend(target_enum.to_token_stream());
    ts.extend(path_sep());
    ts.extend(target_variant.to_token_stream());
    ts.extend(punct(','));

    ts
}

pub fn tuple_to_tuple_arm(
    source_enum: &ContainerIdent,
    source_variant: &VariantIdent,
    target_enum: &ContainerIdent,
    target_variant: &VariantIdent,
    source_fields: &[Ident],
    target_fields: &[Ident],
) -> TokenStream {
    let span = Span::call_site();
    let mut ts = TokenStream::new();

    // source_enum::source_variant(field_0, field_1, ...)
    ts.extend(source_enum.to_token_stream());
    ts.extend(path_sep());
    ts.extend(source_variant.to_token_stream());
    ts.extend(group(Delimiter::Parenthesis, comma_separated_idents(source_fields)));

    // =>
    ts.extend(fat_arrow());

    // target_enum::target_variant(field_0.into(), field_1.into(), ...)
    ts.extend(target_enum.to_token_stream());
    ts.extend(path_sep());
    ts.extend(target_variant.to_token_stream());

    let mut target_args = TokenStream::new();
    for (i, field) in target_fields.iter().enumerate() {
        if i > 0 {
            target_args.extend(punct(','));
        }
        target_args.extend(TokenStream::from(TokenTree::Ident(field.clone())));
        target_args.extend(punct('.'));
        target_args.extend(ident("into", span));
        target_args.extend(group(Delimiter::Parenthesis, TokenStream::new()));
    }
    ts.extend(group(Delimiter::Parenthesis, target_args));
    ts.extend(punct(','));

    ts
}

pub fn struct_to_struct_arm(
    source_enum: &ContainerIdent,
    source_variant: &VariantIdent,
    target_enum: &ContainerIdent,
    target_variant: &VariantIdent,
    source_fields: &[&FieldIdent],
    target_fields: &[(&FieldIdent, &FieldIdent)], // (target_field, source_field_to_convert)
) -> TokenStream {
    let span = Span::call_site();
    let mut ts = TokenStream::new();

    // source_enum::source_variant { field1, field2, ... }
    ts.extend(source_enum.to_token_stream());
    ts.extend(path_sep());
    ts.extend(source_variant.to_token_stream());

    let mut source_args = TokenStream::new();
    for (i, field) in source_fields.iter().enumerate() {
        if i > 0 {
            source_args.extend(punct(','));
        }
        source_args.extend(field.to_token_stream());
    }
    ts.extend(group(Delimiter::Brace, source_args));

    // =>
    ts.extend(fat_arrow());

    // target_enum::target_variant { target_field: source_field.into(), ... }
    ts.extend(target_enum.to_token_stream());
    ts.extend(path_sep());
    ts.extend(target_variant.to_token_stream());

    let mut target_args = TokenStream::new();
    for (i, (target_field, source_field)) in target_fields.iter().enumerate() {
        if i > 0 {
            target_args.extend(punct(','));
        }
        target_args.extend(target_field.to_token_stream());
        target_args.extend(punct(':'));
        target_args.extend(source_field.to_token_stream());
        target_args.extend(punct('.'));
        target_args.extend(ident("into", span));
        target_args.extend(group(Delimiter::Parenthesis, TokenStream::new()));
    }
    ts.extend(group(Delimiter::Brace, target_args));
    ts.extend(punct(','));

    ts
}

pub fn struct_to_tuple_arm(
    source_enum: &ContainerIdent,
    source_variant: &VariantIdent,
    target_enum: &ContainerIdent,
    target_variant: &VariantIdent,
    source_fields: &[&FieldIdent],
    target_fields: &[&FieldIdent], // ordered by target position
) -> TokenStream {
    let span = Span::call_site();
    let mut ts = TokenStream::new();

    // source_enum::source_variant { field1, field2, ... }
    ts.extend(source_enum.to_token_stream());
    ts.extend(path_sep());
    ts.extend(source_variant.to_token_stream());

    let mut source_args = TokenStream::new();
    for (i, field) in source_fields.iter().enumerate() {
        if i > 0 {
            source_args.extend(punct(','));
        }
        source_args.extend(field.to_token_stream());
    }
    ts.extend(group(Delimiter::Brace, source_args));

    // =>
    ts.extend(fat_arrow());

    // target_enum::target_variant(field1.into(), field2.into(), ...)
    ts.extend(target_enum.to_token_stream());
    ts.extend(path_sep());
    ts.extend(target_variant.to_token_stream());

    let mut target_args = TokenStream::new();
    for (i, field) in target_fields.iter().enumerate() {
        if i > 0 {
            target_args.extend(punct(','));
        }
        target_args.extend(field.to_token_stream());
        target_args.extend(punct('.'));
        target_args.extend(ident("into", span));
        target_args.extend(group(Delimiter::Parenthesis, TokenStream::new()));
    }
    ts.extend(group(Delimiter::Parenthesis, target_args));
    ts.extend(punct(','));

    ts
}

pub fn tuple_to_struct_arm(
    source_enum: &ContainerIdent,
    source_variant: &VariantIdent,
    target_enum: &ContainerIdent,
    target_variant: &VariantIdent,
    source_fields: &[&FieldIdent], // field names used for matching tuple positions
    target_fields: &[&FieldIdent], // same field names, used for struct construction
) -> TokenStream {
    let span = Span::call_site();
    let mut ts = TokenStream::new();

    // source_enum::source_variant(field_name1, field_name2, ...)
    ts.extend(source_enum.to_token_stream());
    ts.extend(path_sep());
    ts.extend(source_variant.to_token_stream());

    let mut source_args = TokenStream::new();
    for (i, field) in source_fields.iter().enumerate() {
        if i > 0 {
            source_args.extend(punct(','));
        }
        source_args.extend(field.to_token_stream());
    }
    ts.extend(group(Delimiter::Parenthesis, source_args));

    // =>
    ts.extend(fat_arrow());

    // target_enum::target_variant { field_name1: field_name1.into(), ... }
    ts.extend(target_enum.to_token_stream());
    ts.extend(path_sep());
    ts.extend(target_variant.to_token_stream());

    let mut target_args = TokenStream::new();
    for (i, field) in target_fields.iter().enumerate() {
        if i > 0 {
            target_args.extend(punct(','));
        }
        target_args.extend(field.to_token_stream());
        target_args.extend(punct(':'));
        target_args.extend(field.to_token_stream());
        target_args.extend(punct('.'));
        target_args.extend(ident("into", span));
        target_args.extend(group(Delimiter::Parenthesis, TokenStream::new()));
    }
    ts.extend(group(Delimiter::Brace, target_args));
    ts.extend(punct(','));

    ts
}

fn comma_separated_idents(idents: &[Ident]) -> TokenStream {
    let mut ts = TokenStream::new();
    for (i, id) in idents.iter().enumerate() {
        if i > 0 {
            ts.extend(punct(','));
        }
        ts.extend(TokenStream::from(TokenTree::Ident(id.clone())));
    }
    ts
}
