use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{DeriveInput, parse_quote};
use syn::{Expr, Ident, Token, Type, punctuated::Punctuated};

/// Allow variants of enum to be sampled by specified weights
///
/// # Examples
/// ```
///     #[derive(Sample)]
///     enum BoolOrInst {
///         #[w = 0.1]
///         Bool(BoolInst),
///         #[w = 0.9]
///         Arith(ArithInst)
///     }
/// ```
#[proc_macro_derive(Sample, attributes(w))]
pub fn sample_derive(input: TokenStream) -> TokenStream {
    expand_sample_derive(input).map_or_else(
        |err| err.into_compile_error().into(),
        |expanded| expanded.into(),
    )
}

struct Candidate<'a> {
    ident: &'a Ident,
    ty: &'a Type,
    weight: &'a Expr,
}

fn expand_sample_derive(input: TokenStream) -> syn::Result<TokenStream2> {
    let input = syn::parse::<DeriveInput>(input)?;
    let item_ident = input.ident.clone();
    let syn::Data::Enum(enum_input) = input.data else {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "Sample can only derived for enum",
        ));
    };
    let where_clause = expand_variant_trait_bound(&enum_input.variants)?;

    let mut candidates = vec![];
    for variant in enum_input.variants.iter() {
        let weight = variant
            .attrs
            .iter()
            .find_map(|attr| {
                if let syn::Meta::NameValue(syn::MetaNameValue { path, value, .. }) = &attr.meta {
                    if path.get_ident().is_some_and(|ident| *ident == "w") {
                        Some(value)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .ok_or(syn::Error::new(
                variant.ident.span(),
                "weight to sample this variant should be specified with #[w = value] tag",
            ))?;
        let ty = if let syn::Fields::Unnamed(syn::FieldsUnnamed { unnamed, .. }) = &variant.fields {
            &unnamed.iter().next().unwrap().ty
        } else {
            // checked by expand variant trait bound
            unreachable!()
        };
        candidates.push(Candidate {
            weight,
            ident: &variant.ident,
            ty,
        });
    }
    let sample_trait_path = quote! {crate::dist::Sample};
    let sample_with_ctx_body = variant_sampler(&item_ident, &candidates, |ty| {
        parse_quote! {
            <#ty as #sample_trait_path>::sample_with_ctx(ctx, rng)
        }
    });
    let sample_body = variant_sampler(
        &item_ident,
        &candidates,
        |ty| parse_quote! {<#ty as #sample_trait_path>::sample(rng)},
    );
    Ok(quote! {
        #[automatically_derived]
        impl #sample_trait_path for #item_ident #where_clause {
            fn sample_with_ctx<R: ::rand::Rng + ?Sized>(ctx: &crate::dist::Context, rng: &mut R) -> Self {
                #sample_with_ctx_body
            }

            fn sample<R: ::rand::Rng + ?Sized>(rng: &mut R) -> Self {
                #sample_body
            }
        }
    })
}

fn variant_sampler<F>(enum_ident: &Ident, candidates: &[Candidate<'_>], action: F) -> TokenStream2
where
    F: Fn(&Type) -> Expr,
{
    let weights: Vec<_> = candidates.iter().map(|cand| &cand.weight).collect();
    let num_variants = candidates.len();
    let arms_lhs = 0..num_variants;
    let arms_rhs = candidates.iter().map(|cand| {
        let expr = action(cand.ty);
        let variant_ident = cand.ident;
        quote! {
            #enum_ident::#variant_ident(#expr)
        }
    });
    quote! {
        use ::rand::distr::Distribution;
        let s: Vec<_> = (0..#num_variants).collect();
        let weighted_vec = ::rand::distr::weighted::WeightedIndex::new([#(#weights,)*]).unwrap();
        let arm_idx = s[weighted_vec.sample(rng)];
        match arm_idx {
            #(#arms_lhs => {#arms_rhs})*,
            _ => unreachable!()
        }
    }
}

fn expand_variant_trait_bound(
    variants: &Punctuated<syn::Variant, Token![,]>,
) -> syn::Result<syn::WhereClause> {
    let mut predicates = Punctuated::new();
    for variant in variants.iter() {
        match &variant.fields {
            syn::Fields::Unnamed(syn::FieldsUnnamed { unnamed, .. }) if unnamed.len() == 1 => {
                let ty = &unnamed.iter().next().unwrap().ty;
                predicates.push(parse_quote! {
                    #ty: crate::dist::Sample
                })
            }
            _ => {
                return Err(syn::Error::new(
                    variant.ident.span(),
                    "variant should have one singleton named type",
                ));
            }
        }
    }
    Ok(syn::WhereClause {
        where_token: parse_quote! {where},
        predicates,
    })
}
