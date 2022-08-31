use super::*;
use sp_runtime::offchain::{http, Duration};
use utils::log;

/// Send get request
pub fn get(url: &str) -> Result<String, http::Error> {
    let request = http::Request::get(url);
    execute_request(request)
}

///Send post request with `body` and header Content-Type: application/json
pub fn post(url: &str, body: Vec<&[u8]>) -> Result<String, http::Error> {
    let mut request = http::Request::post(url, body);
    request = request.add_header("Content-type", "application/json");

    execute_request(request)
}

fn execute_request<T: Default + IntoIterator<Item = I>, I: AsRef<[u8]>>(
    request: http::Request<T>,
) -> Result<String, http::Error> {
    let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(5_000));

    let url = request.url.clone();
    let pending = request.deadline(deadline).send().map_err(|e| {
        log::error!(
            "Error sending request. Request: {:?}, deadline: {:?}.",
            url,
            deadline
        );
        match e {
            sp_runtime::offchain::HttpError::DeadlineReached => http::Error::DeadlineReached,
            sp_runtime::offchain::HttpError::IoError => http::Error::IoError,
            sp_runtime::offchain::HttpError::Invalid => http::Error::Unknown,
        }
    })?;
    // no response or a timeout
    let response = pending
        .try_wait(deadline)
        .map_err(|_| {
            log::error!("Didn't receive response. Deadline: {:?}.", deadline);
            http::Error::DeadlineReached
        })?
        .map_err(|e| {
            log::error!("RESPONSE {:?}", e);
            e
        })?;
    if response.code != 200 {
        log::error!("Unexpected status code: {}", response.code);
        return Err(http::Error::Unknown);
    }
    let body = response.body();
    let str = String::from_utf8(body.collect()).unwrap_or(String::new());
    Ok(str)
}
