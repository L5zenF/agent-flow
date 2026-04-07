use rquickjs::{Context, Runtime};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GuestRequest {
    code: String,
    input: Value,
}

#[derive(Debug, Serialize)]
struct GuestResponse {
    ok: bool,
    json: Option<String>,
    error: Option<String>,
}

#[unsafe(no_mangle)]
pub extern "C" fn alloc(len: u32) -> u32 {
    let mut buf = Vec::<u8>::with_capacity(len as usize);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr as u32
}

#[unsafe(no_mangle)]
pub extern "C" fn dealloc(ptr: u32, len: u32) {
    if ptr == 0 || len == 0 {
        return;
    }
    unsafe {
        let _ = Vec::from_raw_parts(ptr as *mut u8, len as usize, len as usize);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn run_json(ptr: u32, len: u32) -> u64 {
    let response = match unsafe { read_bytes(ptr, len) }
        .and_then(|bytes| serde_json::from_slice::<GuestRequest>(&bytes).map_err(|error| error.to_string()))
        .and_then(execute_request)
    {
        Ok(json) => GuestResponse {
            ok: true,
            json: Some(json),
            error: None,
        },
        Err(error) => GuestResponse {
            ok: false,
            json: None,
            error: Some(error),
        },
    };

    write_response(&response)
}

fn execute_request(request: GuestRequest) -> Result<String, String> {
    let runtime = Runtime::new().map_err(|error| format!("failed to initialize QuickJS runtime: {error}"))?;
    let context = Context::full(&runtime)
        .map_err(|error| format!("failed to initialize QuickJS context: {error}"))?;
    let input_json = serde_json::to_string(&request.input)
        .map_err(|error| format!("failed to serialize code runner input: {error}"))?;
    let source = normalize_source(&request.code);

    context.with(|ctx| -> Result<String, String> {
        let globals = ctx.globals();
        globals
            .set("__code_runner_input_json", input_json.as_str())
            .map_err(|error| format!("failed to install code runner input: {error}"))?;
        ctx.eval::<(), _>(source.as_str())
            .map_err(|error| format!("javascript execution failed: {error}"))?;
        ctx.eval::<String, _>(
            r#"
            (() => {
              if (typeof run !== "function") {
                throw new Error("code_runner must define function run(input)");
              }
              const input = JSON.parse(globalThis.__code_runner_input_json);
              return JSON.stringify(run(input) ?? {});
            })()
            "#,
        )
        .map_err(|error| format!("javascript execution failed: {error}"))
    })
}

fn normalize_source(source: &str) -> String {
    let trimmed = source.trim_start();
    if trimmed.starts_with("export function run") {
        source.replacen("export function run", "function run", 1)
    } else {
        source.to_string()
    }
}

unsafe fn read_bytes(ptr: u32, len: u32) -> Result<Vec<u8>, String> {
    if ptr == 0 {
        return Err("received null input pointer".to_string());
    }
    Ok(unsafe { std::slice::from_raw_parts(ptr as *const u8, len as usize) }.to_vec())
}

fn write_response(response: &GuestResponse) -> u64 {
    let bytes = serde_json::to_vec(response).unwrap_or_else(|error| {
        format!(r#"{{"ok":false,"error":"failed to serialize guest response: {error}"}}"#).into_bytes()
    });
    if bytes.is_empty() {
        return 0;
    }

    let len = bytes.len() as u32;
    let ptr = alloc(len);
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr as *mut u8, len as usize);
    }
    ((len as u64) << 32) | (ptr as u64)
}
