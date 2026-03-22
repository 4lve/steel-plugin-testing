use proc_macro::TokenStream;
use quote::quote;
use syn::{FnArg, ItemFn, Pat, parse_macro_input};

/// Wraps a plugin init function with `catch_unwind` so panics never
/// cross the FFI boundary.
///
/// ```ignore
/// #[steel_plugin]
/// fn init(ctx: &PluginContext) {
///     ctx.commands().register("hello", hello_command);
/// }
/// ```
#[proc_macro_attribute]
pub fn steel_plugin(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);
    let fn_name = &input_fn.sig.ident;
    let fn_block = &input_fn.block;
    let fn_args = &input_fn.sig.inputs;
    let fn_attrs = &input_fn.attrs;

    if fn_args.len() != 1 {
        return syn::Error::new_spanned(
            fn_args,
            "#[steel_plugin] function must take exactly one argument: `ctx: &PluginContext`",
        )
        .to_compile_error()
        .into();
    }

    let output = quote! {
        #(#fn_attrs)*
        #[stabby::export]
        pub extern "C" fn steel_plugin_init(
            __steel_ctx: steel_api::PluginContext,
        ) -> steel_api::InitResult {
            match ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(|| {
                #fn_name(&__steel_ctx);
            })) {
                Ok(()) => steel_api::InitResult::Ok,
                Err(__e) => {
                    steel_api::log_panic("init", &*__e);
                    steel_api::InitResult::Panic
                }
            }
        }

        fn #fn_name(#fn_args) #fn_block
    };

    output.into()
}

/// Wraps an event handler with `catch_unwind` and makes it `extern "C"`.
/// Panics are caught and returned as `EventResult::Panic`.
///
/// ```ignore
/// #[steel_handler]
/// fn on_event(event: &mut PlayerJoinEvent, cancelled: &mut bool) -> EventResult {
///     *cancelled = true; // cancel the event
///     EventResult::Continue
/// }
/// ```
#[proc_macro_attribute]
pub fn steel_handler(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);
    let fn_name = &input_fn.sig.ident;
    let fn_block = &input_fn.block;
    let fn_args = &input_fn.sig.inputs;
    let fn_ret = &input_fn.sig.output;
    let fn_attrs = &input_fn.attrs;

    if fn_args.len() != 2 {
        return syn::Error::new_spanned(
            fn_args,
            "#[steel_handler] function must take exactly 2 arguments: \
             `(event: &mut Event, cancelled: &mut bool)`",
        )
        .to_compile_error()
        .into();
    }

    let arg_names: Vec<_> = fn_args
        .iter()
        .filter_map(|arg| {
            if let FnArg::Typed(pat_type) = arg {
                if let Pat::Ident(pat_ident) = pat_type.pat.as_ref() {
                    return Some(&pat_ident.ident);
                }
            }
            None
        })
        .collect();

    let first = &arg_names[0];
    let second = &arg_names[1];

    let output = quote! {
        #(#fn_attrs)*
        extern "C" fn #fn_name(#fn_args) #fn_ret {
            fn __inner(#fn_args) #fn_ret #fn_block

            match ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(|| {
                __inner(#first, #second)
            })) {
                Ok(__r) => __r,
                Err(__e) => {
                    steel_api::log_panic(stringify!(#fn_name), &*__e);
                    steel_api::EventResult::Panic
                }
            }
        }
    };

    output.into()
}

/// Wraps a command handler with `catch_unwind` and makes it `extern "C"`.
///
/// The generated `extern "C"` function takes `CommandContext` by value.
/// The plugin developer's function receives `&CommandContext`.
///
/// ```ignore
/// #[steel_command]
/// fn hello(ctx: &CommandContext) {
///     ctx.reply("Hello!");
/// }
/// ```
#[proc_macro_attribute]
pub fn steel_command(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);
    let fn_name = &input_fn.sig.ident;
    let fn_block = &input_fn.block;
    let fn_args = &input_fn.sig.inputs;
    let fn_attrs = &input_fn.attrs;

    if fn_args.len() != 1 {
        return syn::Error::new_spanned(
            fn_args,
            "#[steel_command] function must take exactly one argument: `ctx: &CommandContext`",
        )
        .to_compile_error()
        .into();
    }

    // Extract the argument name from the user's function signature.
    let arg_name = fn_args
        .iter()
        .find_map(|arg| {
            if let FnArg::Typed(pat_type) = arg {
                if let Pat::Ident(pat_ident) = pat_type.pat.as_ref() {
                    return Some(pat_ident.ident.clone());
                }
            }
            None
        })
        .expect("#[steel_command] argument must be a simple identifier");

    let output = quote! {
        #(#fn_attrs)*
        extern "C" fn #fn_name(__steel_ctx: steel_api::CommandContext) -> steel_api::CommandResult {
            fn __inner(#fn_args) #fn_block

            let #arg_name = &__steel_ctx;
            match ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(|| {
                __inner(#arg_name)
            })) {
                Ok(()) => steel_api::CommandResult::Ok,
                Err(__e) => {
                    steel_api::log_panic(
                        concat!("command:", stringify!(#fn_name)),
                        &*__e,
                    );
                    steel_api::CommandResult::Panic
                }
            }
        }
    };

    output.into()
}
