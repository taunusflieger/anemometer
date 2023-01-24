/*
 * ESP32 Anemometer
 *
 * MIT license
 *
 * Copyright (c) 2021-2023 Michael Zill
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 *
 * Apache license, Version 2.0
 *
 * Copyright (c) 2021-2023 Michael Zill
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
use super::cstr::*;
use core::ptr;
use esp_idf_svc::{handle::RawHandle, nvs::*};
use esp_idf_sys::*;

pub trait EspNvsExtention {
    fn len_str(&self, name: &str) -> Result<Option<usize>, EspError>;
    fn get_str<'a>(&self, name: &str, buf: &'a mut [u8]) -> Result<Option<&'a [u8]>, EspError>;
    fn set_str(&mut self, name: &str, val: &str) -> Result<bool, EspError>;
    fn get_u8<'a>(&self, name: &str, out_val: &'a mut u8) -> Result<Option<&'a u8>, EspError>;
    fn set_u8(&self, name: &str, val: u8) -> Result<bool, EspError>;
    fn get_i8<'a>(&self, name: &str, out_val: &'a mut i8) -> Result<Option<&'a i8>, EspError>;
    fn set_i8(&self, name: &str, val: i8) -> Result<bool, EspError>;
    fn get_u16<'a>(&self, name: &str, out_val: &'a mut u16) -> Result<Option<&'a u16>, EspError>;
    fn set_u16(&self, name: &str, val: u16) -> Result<bool, EspError>;
    fn get_i16<'a>(&self, name: &str, out_val: &'a mut i16) -> Result<Option<&'a i16>, EspError>;
    fn set_i16(&self, name: &str, val: i16) -> Result<bool, EspError>;
    fn get_u32<'a>(&self, name: &str, out_val: &'a mut u32) -> Result<Option<&'a u32>, EspError>;
    fn set_u32(&self, name: &str, val: u32) -> Result<bool, EspError>;
    fn get_i32<'a>(&self, name: &str, out_val: &'a mut i32) -> Result<Option<&'a i32>, EspError>;
    fn set_i32(&self, name: &str, val: i32) -> Result<bool, EspError>;
    fn get_u64<'a>(&self, name: &str, out_val: &'a mut u64) -> Result<Option<&'a u64>, EspError>;
    fn set_u64(&self, name: &str, val: u64) -> Result<bool, EspError>;
    fn get_i64<'a>(&self, name: &str, out_val: &'a mut i64) -> Result<Option<&'a i64>, EspError>;
    fn set_i64(&self, name: &str, val: i64) -> Result<bool, EspError>;
}

impl EspNvsExtention for EspCustomNvs {
    fn len_str(&self, name: &str) -> Result<Option<usize>, EspError> {
        let c_key = CString::new(name).unwrap();

        #[allow(unused_assignments)]
        let mut len = 0;

        match unsafe {
            nvs_get_str(
                self.handle(),
                c_key.as_ptr(),
                ptr::null_mut(),
                &mut len as *mut _,
            )
        } {
            ESP_ERR_NVS_NOT_FOUND => Ok(None),
            err => {
                // bail on error
                esp!(err)?;

                Ok(Some(len))
            }
        }
    }

    fn get_str<'a>(&self, name: &str, buf: &'a mut [u8]) -> Result<Option<&'a [u8]>, EspError> {
        let c_key = CString::new(name).unwrap();

        #[allow(unused_assignments)]
        let mut len = 0;
        match unsafe {
            len = buf.len();
            nvs_get_str(
                self.handle(),
                c_key.as_ptr(),
                buf.as_mut_ptr() as *mut _,
                &mut len as *mut _,
            )
        } {
            ESP_ERR_NVS_NOT_FOUND => Ok(None),
            err => {
                // bail on error
                esp!(err)?;

                Ok(Some(buf))
            }
        }
    }

    fn set_str(&mut self, name: &str, val: &str) -> Result<bool, EspError> {
        let c_key = CString::new(name).unwrap();
        let c_val = CString::new(val).unwrap();

        // start by just clearing this key
        unsafe { nvs_erase_key(self.handle(), c_key.as_ptr()) };

        esp!(unsafe { nvs_set_str(self.handle(), c_key.as_ptr(), c_val.as_ptr(),) })?;

        esp!(unsafe { nvs_commit(self.handle()) })?;

        Ok(true)
    }

    fn get_u8<'a>(&self, name: &str, out_val: &'a mut u8) -> Result<Option<&'a u8>, EspError> {
        let c_key = CString::new(name).unwrap();

        match unsafe { nvs_get_u8(self.handle(), c_key.as_ptr(), out_val as *mut _) } {
            ESP_ERR_NVS_NOT_FOUND => Ok(None),
            err => {
                // bail on error
                esp!(err)?;

                Ok(Some(out_val))
            }
        }
    }

    fn set_u8(&self, name: &str, val: u8) -> Result<bool, EspError> {
        let c_key = CString::new(name).unwrap();

        esp!(unsafe { nvs_set_u8(self.handle(), c_key.as_ptr(), val) })?;

        Ok(true)
    }

    fn get_i8<'a>(&self, name: &str, out_val: &'a mut i8) -> Result<Option<&'a i8>, EspError> {
        let c_key = CString::new(name).unwrap();
        match unsafe { nvs_get_i8(self.handle(), c_key.as_ptr(), out_val as *mut _) } {
            ESP_ERR_NVS_NOT_FOUND => Ok(None),
            err => {
                // bail on error
                esp!(err)?;

                Ok(Some(out_val))
            }
        }
    }

    fn set_i8(&self, name: &str, val: i8) -> Result<bool, EspError> {
        let c_key = CString::new(name).unwrap();

        esp!(unsafe { nvs_set_i8(self.handle(), c_key.as_ptr(), val) })?;

        Ok(true)
    }

    fn get_u16<'a>(&self, name: &str, out_val: &'a mut u16) -> Result<Option<&'a u16>, EspError> {
        let c_key = CString::new(name).unwrap();
        match unsafe { nvs_get_u16(self.handle(), c_key.as_ptr(), out_val as *mut _) } {
            ESP_ERR_NVS_NOT_FOUND => Ok(None),
            err => {
                // bail on error
                esp!(err)?;

                Ok(Some(out_val))
            }
        }
    }

    fn set_u16(&self, name: &str, val: u16) -> Result<bool, EspError> {
        let c_key = CString::new(name).unwrap();

        esp!(unsafe { nvs_set_u16(self.handle(), c_key.as_ptr(), val) })?;

        Ok(true)
    }

    fn get_i16<'a>(&self, name: &str, out_val: &'a mut i16) -> Result<Option<&'a i16>, EspError> {
        let c_key = CString::new(name).unwrap();
        match unsafe { nvs_get_i16(self.handle(), c_key.as_ptr(), out_val as *mut _) } {
            ESP_ERR_NVS_NOT_FOUND => Ok(None),
            err => {
                // bail on error
                esp!(err)?;

                Ok(Some(out_val))
            }
        }
    }

    fn set_i16(&self, name: &str, val: i16) -> Result<bool, EspError> {
        let c_key = CString::new(name).unwrap();

        esp!(unsafe { nvs_set_i16(self.handle(), c_key.as_ptr(), val) })?;

        Ok(true)
    }

    fn get_u32<'a>(&self, name: &str, out_val: &'a mut u32) -> Result<Option<&'a u32>, EspError> {
        let c_key = CString::new(name).unwrap();
        match unsafe { nvs_get_u32(self.handle(), c_key.as_ptr(), out_val as *mut _) } {
            ESP_ERR_NVS_NOT_FOUND => Ok(None),
            err => {
                // bail on error
                esp!(err)?;

                Ok(Some(out_val))
            }
        }
    }

    fn set_u32(&self, name: &str, val: u32) -> Result<bool, EspError> {
        let c_key = CString::new(name).unwrap();

        esp!(unsafe { nvs_set_u32(self.handle(), c_key.as_ptr(), val) })?;

        Ok(true)
    }

    fn get_i32<'a>(&self, name: &str, out_val: &'a mut i32) -> Result<Option<&'a i32>, EspError> {
        let c_key = CString::new(name).unwrap();
        match unsafe { nvs_get_i32(self.handle(), c_key.as_ptr(), out_val as *mut _) } {
            ESP_ERR_NVS_NOT_FOUND => Ok(None),
            err => {
                // bail on error
                esp!(err)?;

                Ok(Some(out_val))
            }
        }
    }

    fn set_i32(&self, name: &str, val: i32) -> Result<bool, EspError> {
        let c_key = CString::new(name).unwrap();

        esp!(unsafe { nvs_set_i32(self.handle(), c_key.as_ptr(), val) })?;

        Ok(true)
    }

    fn get_u64<'a>(&self, name: &str, out_val: &'a mut u64) -> Result<Option<&'a u64>, EspError> {
        let c_key = CString::new(name).unwrap();
        match unsafe { nvs_get_u64(self.handle(), c_key.as_ptr(), out_val as *mut _) } {
            ESP_ERR_NVS_NOT_FOUND => Ok(None),
            err => {
                // bail on error
                esp!(err)?;

                Ok(Some(out_val))
            }
        }
    }

    fn set_u64(&self, name: &str, val: u64) -> Result<bool, EspError> {
        let c_key = CString::new(name).unwrap();

        esp!(unsafe { nvs_set_u64(self.handle(), c_key.as_ptr(), val) })?;

        Ok(true)
    }

    fn get_i64<'a>(&self, name: &str, out_val: &'a mut i64) -> Result<Option<&'a i64>, EspError> {
        let c_key = CString::new(name).unwrap();
        match unsafe { nvs_get_i64(self.handle(), c_key.as_ptr(), out_val as *mut _) } {
            ESP_ERR_NVS_NOT_FOUND => Ok(None),
            err => {
                // bail on error
                esp!(err)?;

                Ok(Some(out_val))
            }
        }
    }

    fn set_i64(&self, name: &str, val: i64) -> Result<bool, EspError> {
        let c_key = CString::new(name).unwrap();

        esp!(unsafe { nvs_set_i64(self.handle(), c_key.as_ptr(), val) })?;

        Ok(true)
    }
}
