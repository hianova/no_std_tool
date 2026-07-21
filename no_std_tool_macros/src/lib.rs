extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::visit::Visit;
use syn::{Error, ItemStruct, Lit, parse_macro_input};
use std::str::FromStr;

struct AllocVisitor {
    errors: Vec<Error>,
}

impl<'ast> Visit<'ast> for AllocVisitor {
    fn visit_ident(&mut self, i: &'ast syn::Ident) {
        let name = i.to_string();
        if name == "alloc"
            || name == "Box"
            || name == "Vec"
            || name == "String"
            || name == "Rc"
            || name == "Arc"
        {
            self.errors.push(Error::new_spanned(
                i,
                format!(
                    "Forbidden token '{}' found! Allocation is not allowed in auto_static structs.",
                    name
                ),
            ));
        }
        syn::visit::visit_ident(self, i);
    }
}

#[proc_macro_attribute]
pub fn auto_static(args: TokenStream, input: TokenStream) -> TokenStream {
    let input_ast = parse_macro_input!(input as ItemStruct);
    let mut visitor = AllocVisitor { errors: Vec::new() };
    visitor.visit_item_struct(&input_ast);
    if let Some(first_err) = visitor.errors.into_iter().next() {
        return first_err.to_compile_error().into();
    }
    let mut capacity: Option<usize> = None;
    let mut partition: Option<String> = None;
    let meta_parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("capacity") {
            let value: Lit = meta.value()?.parse()?;
            if let Lit::Int(int_lit) = value {
                capacity = Some(int_lit.base10_parse::<usize>()?);
            }
            Ok(())
        } else if meta.path.is_ident("partition") {
            let value: Lit = meta.value()?.parse()?;
            if let Lit::Str(str_lit) = value {
                partition = Some(str_lit.value());
            }
            Ok(())
        } else {
            Err(meta.error("unsupported auto_static property"))
        }
    });
    parse_macro_input!(args with meta_parser);
    let capacity = match capacity {
        Some(c) => c,
        None => {
            return Error::new_spanned(&input_ast.ident, "Missing `capacity` attribute")
                .to_compile_error()
                .into();
        }
    };
    let partition = partition.unwrap_or_else(|| "default_pool".to_string());
    let section_name = format!(".bss.{}", partition);
    let struct_name = &input_ast.ident;
    let pool_name = format_ident!("{}_POOL", struct_name.to_string().to_uppercase());
    let bitmap_name = format_ident!("{}_BITMAP", struct_name.to_string().to_uppercase());
    let camel_partition: String = partition
        .split('_')
        .map(|word| {
            let mut c = word.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect();
    let token_name = format_ident!("{}Token", camel_partition);
    let num_bitmap_words = capacity.div_ceil(64);
    let expanded = quote! {
        #input_ast
        #[cfg_attr(target_vendor = "apple", unsafe(link_section = concat!("__DATA,", #partition)))]
        #[cfg_attr(not(target_vendor = "apple"), unsafe(link_section = #section_name))]
        static mut #pool_name: [core::mem::MaybeUninit<#struct_name>; #capacity] = [const { core::mem::MaybeUninit::uninit() }; #capacity];
        #[cfg_attr(target_vendor = "apple", unsafe(link_section = concat!("__DATA,", #partition)))]
        #[cfg_attr(not(target_vendor = "apple"), unsafe(link_section = #section_name))]
        static mut #bitmap_name: [u64; #num_bitmap_words] = [0; #num_bitmap_words];
        impl #struct_name {
            pub fn insert(val: Self, _token: &mut #token_name) -> Option<usize> {
                unsafe {
                    for i in 0..#num_bitmap_words {
                        let word = #bitmap_name[i];
                        if word != u64::MAX {
                            let free_bit = (!word).trailing_zeros() as usize;
                            let slot_idx = i * 64 + free_bit;
                            if slot_idx < #capacity {
                                #bitmap_name[i] |= 1 << free_bit;
                                #pool_name[slot_idx].as_mut_ptr().write(val);
                                return Some(slot_idx);
                            }
                        }
                    }
                }
                None
            }
            pub fn remove(index: usize, _token: &mut #token_name) -> Option<Self> {
                if index >= #capacity { return None; }
                let word_idx = index / 64;
                let bit_idx = index % 64;
                unsafe {
                    let word = #bitmap_name[word_idx];
                    if (word & (1 << bit_idx)) != 0 {
                        #bitmap_name[word_idx] &= !(1 << bit_idx);
                        let val = #pool_name[index].as_ptr().read();
                        Some(val)
                    } else { None }
                }
            }
            pub fn get(index: usize, _token: &#token_name) -> Option<&Self> {
                if index >= #capacity { return None; }
                let word_idx = index / 64;
                let bit_idx = index % 64;
                unsafe {
                    let word = #bitmap_name[word_idx];
                    if (word & (1 << bit_idx)) != 0 {
                        Some(&*#pool_name[index].as_ptr())
                    } else { None }
                }
            }
            pub fn get_mut(index: usize, _token: &mut #token_name) -> Option<&mut Self> {
                if index >= #capacity { return None; }
                let word_idx = index / 64;
                let bit_idx = index % 64;
                unsafe {
                    let word = #bitmap_name[word_idx];
                    if (word & (1 << bit_idx)) != 0 {
                        Some(&mut *#pool_name[index].as_mut_ptr())
                    } else { None }
                }
            }
        }
    };
    TokenStream::from(expanded)
}

/// AOT transpile ScriptGo (SGL) into zero-cost Rust loops at compile time!
#[proc_macro]
pub fn sgl_compile(input: TokenStream) -> TokenStream {
    let input_lit = parse_macro_input!(input as Lit);
    if let Lit::Str(str_lit) = input_lit {
        let mut sgl_code = str_lit.value();
        
        // Transpile SGL to Rust
        // Basic transpilation rules for zero-cost iterators:
        sgl_code = sgl_code.replace("let ", "let mut ");
        sgl_code = sgl_code.replace(": Int", ": u32");
        sgl_code = sgl_code.replace(": Float", ": f64");
        
        match TokenStream::from_str(&sgl_code) {
            Ok(ts) => ts,
            Err(e) => {
                let err = format!("Failed to parse transpiled SGL: {:?}", e);
                let err_ts = quote! { compile_error!(#err); };
                TokenStream::from(err_ts)
            }
        }
    } else {
        TokenStream::from(quote! { compile_error!("Expected a string literal containing SGL code"); })
    }
}
