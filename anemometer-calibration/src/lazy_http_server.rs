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
pub mod lazy_init_http_server {
    use std::cell::{RefCell, RefMut};
    use std::rc::Rc;

    use esp_idf_svc::http::server::{Configuration, EspHttpServer};

    pub struct LazyInitHttpServer {
        data: Rc<RefCell<Option<EspHttpServer>>>,
    }

    impl LazyInitHttpServer {
        pub fn new() -> Self {
            Self {
                data: Rc::new(RefCell::new(None)),
            }
        }
        pub fn create(&self, conf: &Configuration) -> RefMut<'_, EspHttpServer> {
            if self.data.borrow().is_none() {
                *self.data.borrow_mut() = Some(EspHttpServer::new(conf).unwrap());
            }
            let m = self.data.borrow_mut();
            RefMut::map(m, |m| m.as_mut().unwrap())
        }
        /*
        pub fn get(&self) -> Option<RefMut<'_, EspHttpServer>> {
            let m = self.data.borrow_mut();
            if m.is_some() {
                Some(RefMut::map(m, |m| m.as_mut().unwrap()))
            } else {
                None
            }
        }
        */
        pub fn clear(&self) {
            *self.data.borrow_mut() = None;
        }
        /*
        fn ref_count(&self) -> usize {
            Rc::strong_count(&self.data)
        }
        */
    }
}
