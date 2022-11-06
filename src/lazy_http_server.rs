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
