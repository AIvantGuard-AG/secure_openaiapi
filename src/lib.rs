use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList};
use reqwest::blocking::Client;
use serde::ser::{SerializeStruct, Serializer};
use serde::{Deserialize, Serialize};
use std::ffi::c_void;
use std::str;
use libsodium_sys::{sodium_init, sodium_mlock, sodium_munlock};
use zeroize::{Zeroize, ZeroizeOnDrop};

// --- SecureBytes Wrapper ---
#[pyclass(name = "SecureBytes")]
#[derive(Zeroize, Clone, Debug)]
pub struct SecureBytes {
    inner: Vec<u8>,
}

impl SecureBytes {
    pub fn new(data: &[u8]) -> Self {
        let mut inner = data.to_vec();
        unsafe {
            if sodium_init() < 0 {
                panic!("Failed to initialize libsodium");
            }
            if !inner.is_empty() {
                sodium_mlock(inner.as_mut_ptr() as *mut c_void, inner.len());
            }
        }
        Self { inner }
    }
    pub fn as_str(&self) -> Result<&str, PyErr> {
        str::from_utf8(&self.inner).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyUnicodeDecodeError, _>(format!("UTF-8 decode error: {}", e))
        })
    }
}

impl Drop for SecureBytes {
    fn drop(&mut self) {
        self.inner.zeroize();
        unsafe {
            if !self.inner.is_empty() {
                sodium_munlock(self.inner.as_mut_ptr() as *mut c_void, self.inner.len());
            }
        }
    }
}

#[pymethods]
impl SecureBytes {
    #[new]
    fn pynew(data: &[u8]) -> Self { Self::new(data) }

    fn __bytes__<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.inner)
    }
    fn __str__(&self) -> PyResult<String> { Ok(self.as_str()?.to_string()) }
    fn __repr__(&self) -> String { "SecureBytes(b'****')".to_string() }
}

impl Serialize for SecureBytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = str::from_utf8(&self.inner).map_err(serde::ser::Error::custom)?;
        serializer.serialize_str(s)
    }
}

// --- API Message Structures (Internal & Serializable) ---
#[derive(Serialize, Clone, Debug, Zeroize, ZeroizeOnDrop)]
struct ImageUrlDetail {
    url: SecureBytes,
}

#[derive(Serialize, Clone, Debug, Zeroize, ZeroizeOnDrop)]
#[serde(tag = "type", rename_all = "snake_case")]
enum SecureContentPart {
    Text { text: SecureBytes },
    ImageUrl { image_url: ImageUrlDetail },
}

#[pyclass(name = "SecureMessage")]
#[derive(Clone, Debug, Zeroize, ZeroizeOnDrop)]
pub struct SecureMessage {
    role: SecureBytes,
    content: Vec<SecureContentPart>,
}

// FIX: Custom Serialize implementation for SecureMessage to handle the API's
// requirement for string content in simple text messages.
impl Serialize for SecureMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("SecureMessage", 2)?;
        state.serialize_field("role", &self.role)?;

        // If content has one item and it's text, serialize it as a plain string.
        // Otherwise, serialize it as a list of content parts.
        if self.content.len() == 1 {
            if let Some(SecureContentPart::Text { text }) = self.content.first() {
                state.serialize_field("content", text)?;
            } else {
                state.serialize_field("content", &self.content)?;
            }
        } else {
            state.serialize_field("content", &self.content)?;
        }

        state.end()
    }
}


#[pymethods]
impl SecureMessage {
    #[new]
    fn new(_py: Python, role: &[u8], content_list: &Bound<PyList>) -> PyResult<Self> {
        let mut content: Vec<SecureContentPart> = Vec::new();

        for item in content_list.iter() {
            let dict: &Bound<PyDict> = item.downcast()?;

            let type_obj = dict
                .get_item("type")?
                .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyKeyError, _>("'type' key missing in content part"))?;
            let content_type: String = type_obj.extract()?;

            match content_type.as_str() {
                "text" => {
                    let text_item = dict
                        .get_item("text")?
                        .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyKeyError, _>("'text' key missing for type 'text'"))?;
                    let text_bytes: &Bound<PyBytes> = text_item.downcast()?;
                    content.push(SecureContentPart::Text {
                        text: SecureBytes::new(text_bytes.as_bytes()),
                    });
                }
                "image_url" => {
                    let image_url_item = dict
                        .get_item("image_url")?
                        .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyKeyError, _>("'image_url' key missing for type 'image_url'"))?;
                    let image_url_dict: &Bound<PyDict> = image_url_item.downcast()?;

                    let url_item = image_url_dict
                        .get_item("url")?
                        .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyKeyError, _>("'url' key missing in image_url object"))?;
                    let url_bytes: &Bound<PyBytes> = url_item.downcast()?;

                    content.push(SecureContentPart::ImageUrl {
                        image_url: ImageUrlDetail {
                            url: SecureBytes::new(url_bytes.as_bytes()),
                        },
                    });
                }
                _ => return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Unsupported content type: {}", content_type))),
            }
        }

        Ok(SecureMessage {
            role: SecureBytes::new(role),
            content,
        })
    }
}


// --- API Request/Response Structs ---

#[derive(Serialize, Debug)]
struct ChatCompletionRequest<'a> {
    messages: &'a Vec<SecureMessage>,
    model: &'a str,
}

#[derive(Deserialize, Debug)]
struct ResponseChoice {
    message: ResponseMessage,
}

#[derive(Deserialize, Debug)]
struct ResponseMessage {
    content: Option<String>,
}

#[derive(Deserialize, Debug)]
struct ChatCompletionResponse {
    choices: Vec<ResponseChoice>,
}

// --- SecureClient ---

#[pyclass(name = "SecureClient")]
struct SecureClient {
    base_url: SecureBytes,
    api_key: SecureBytes,
    http_client: Client,
}

#[pymethods]
impl SecureClient {
    #[new]
    fn new(base_url: &[u8], api_key: &[u8]) -> PyResult<Self> {
        Ok(Self {
            base_url: SecureBytes::new(base_url),
            api_key: SecureBytes::new(api_key),
            http_client: Client::new(),
        })
    }

    #[pyo3(signature = (messages, model))]
    fn chat_completion(&self, messages: Vec<PyRef<SecureMessage>>, model: String) -> PyResult<SecureBytes> {
        let messages_rs: Vec<SecureMessage> = messages.iter().map(|m| (**m).clone()).collect();
        let request_body = ChatCompletionRequest {
            messages: &messages_rs,
            model: &model,
        };

        let base_url_str = self.base_url.as_str()?;
        let api_key_str = self.api_key.as_str()?;
        let endpoint = format!("{}{}", base_url_str, "/openai/v1/chat/completions");

        let response = self.http_client.post(&endpoint).bearer_auth(api_key_str).json(&request_body).send();

        match response {
            Ok(res) => {
                if res.status().is_success() {
                    let body: ChatCompletionResponse = res.json().map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Failed to parse JSON response: {}", e)))?;
                    if let Some(choice) = body.choices.get(0) {
                        let content = choice.message.content.as_deref().unwrap_or("");
                        Ok(SecureBytes::new(content.as_bytes()))
                    } else {
                        Err(PyErr::new::<pyo3::exceptions::PyValueError, _>("API returned no choices."))
                    }
                } else {
                    let status = res.status();
                    let error_body = res.text().unwrap_or_else(|_| "Could not read error body".to_string());
                    Err(PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("API request failed with status {}: {}", status, error_body)))
                }
            }
            Err(e) => Err(PyErr::new::<pyo3::exceptions::PyConnectionError, _>(format!("Failed to send request: {}", e))),
        }
    }
}

// --- Python Module Definition ---

#[pymodule]
fn secure_openaiapi(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<SecureClient>()?;
    m.add_class::<SecureBytes>()?;
    m.add_class::<SecureMessage>()?;
    Ok(())
}
