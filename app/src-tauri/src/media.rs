use crate::{
    projection::Session,
    store::{media_path, Kind, Store},
};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
    sync::Mutex,
};
use tauri::{
    http::{header, Method, Request, Response, StatusCode},
    Manager, UriSchemeContext, UriSchemeResponder, Wry,
};
use uuid::Uuid;

type MediaResponse = Response<Vec<u8>>;

pub fn serve(
    context: UriSchemeContext<'_, Wry>,
    request: Request<Vec<u8>>,
    responder: UriSchemeResponder,
) {
    let app = context.app_handle().clone();
    let label = context.webview_label().to_owned();
    tauri::async_runtime::spawn_blocking(move || {
        let response = match authorize(&app, &label, request.uri().path()) {
            Some((path, kind)) => read(&path, kind, &request),
            None => error(StatusCode::NOT_FOUND),
        };
        responder.respond(response);
    });
}

fn authorize(app: &tauri::AppHandle, label: &str, key: &str) -> Option<(PathBuf, Kind)> {
    let (ids, extension) = key.strip_prefix('/')?.split_once('.')?;
    let (group, id) = ids.split_once('_')?;
    let group = Uuid::parse_str(group).ok()?;
    let id = Uuid::parse_str(id).ok()?;
    let kind = Kind::from_extension(extension)?;
    if label != "main"
        && !app
            .try_state::<Mutex<Session>>()?
            .lock()
            .ok()?
            .allows_media(label, group, id)
    {
        return None;
    }
    let path = media_path(&app.try_state::<Store>()?.root, group, id, kind).ok()?;
    Some((path, kind))
}

fn range(value: &str, size: u64) -> Option<(u64, u64)> {
    let (start, end) = value.strip_prefix("bytes=")?.split_once('-')?;
    if size == 0 {
        return None;
    }
    let (start, end) = if start.is_empty() {
        let suffix: u64 = end.parse().ok()?;
        (size.saturating_sub(suffix), size - 1)
    } else {
        (
            start.parse().ok()?,
            if end.is_empty() {
                size - 1
            } else {
                end.parse::<u64>().ok()?.min(size - 1)
            },
        )
    };
    if start > end {
        return None;
    }
    // Bound allocations for open-ended media reads. Players can request the following range.
    Some((start, end.min(start + 8 * 1024 * 1024 - 1)))
}

fn read(path: &PathBuf, kind: Kind, request: &Request<Vec<u8>>) -> MediaResponse {
    if request.method() != Method::GET && request.method() != Method::HEAD {
        return error(StatusCode::METHOD_NOT_ALLOWED);
    }
    let Ok(mut file) = File::open(path) else {
        return error(StatusCode::NOT_FOUND);
    };
    let Ok(metadata) = file.metadata() else {
        return error(StatusCode::NOT_FOUND);
    };
    let size = metadata.len();
    if !(12..=256 * 1024 * 1024).contains(&size) {
        return error(StatusCode::NOT_FOUND);
    }
    let mut response = Response::builder()
        .header(header::CONTENT_TYPE, kind.mime())
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CACHE_CONTROL, "no-store")
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(header::ACCESS_CONTROL_EXPOSE_HEADERS, "content-range")
        .header("X-Content-Type-Options", "nosniff");
    let (start, count) = if let Some(value) = request.headers().get(header::RANGE) {
        let Some((start, end)) = value.to_str().ok().and_then(|value| range(value, size)) else {
            return response
                .status(StatusCode::RANGE_NOT_SATISFIABLE)
                .header(header::CONTENT_RANGE, format!("bytes */{size}"))
                .body(Vec::new())
                .unwrap_or_else(|_| error(StatusCode::INTERNAL_SERVER_ERROR));
        };
        response = response
            .status(StatusCode::PARTIAL_CONTENT)
            .header(header::CONTENT_RANGE, format!("bytes {start}-{end}/{size}"));
        (start, end - start + 1)
    } else {
        (0, size)
    };
    let mut bytes = Vec::new();
    if request.method() == Method::GET
        && (file.seek(SeekFrom::Start(start)).is_err()
            || file.take(count).read_to_end(&mut bytes).is_err()
            || bytes.len() as u64 != count)
    {
        return error(StatusCode::NOT_FOUND);
    }
    response
        .header(header::CONTENT_LENGTH, count)
        .body(bytes)
        .unwrap_or_else(|_| error(StatusCode::INTERNAL_SERVER_ERROR))
}

fn error(status: StatusCode) -> MediaResponse {
    let mut response = Response::new(Vec::new());
    *response.status_mut() = status;
    response
}

#[cfg(test)]
mod tests {
    use super::range;

    #[test]
    fn single_byte_ranges_are_bounded_and_validated() {
        assert_eq!(range("bytes=0-1", 30), Some((0, 1)));
        assert_eq!(range("bytes=25-", 30), Some((25, 29)));
        assert_eq!(range("bytes=-5", 30), Some((25, 29)));
        assert_eq!(range("bytes=30-", 30), None);
        assert_eq!(range("bytes=2-1", 30), None);
        assert_eq!(range("bytes=0-1,8-9", 30), None);
        assert_eq!(
            range("bytes=0-", 30 * 1024 * 1024),
            Some((0, 8 * 1024 * 1024 - 1))
        );
    }
}
