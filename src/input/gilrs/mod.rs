/*fn main() {
    let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();*/
extern crate gilrs;

use self::gilrs::{Gilrs, Button, Event};

use input::InputBackendInfo;
use input::InputBackend;

struct GLFWBackend {

}

impl InputBackend for GLFWBackend {
    fn poll_events(&mut self) {
        unimplemented!()
    }

    fn is_key_down(&self, key: &InputKey) -> bool {
        unimplemented!()
    }
}

pub static INFO : InputBackendInfo = InputBackendInfo {
    name: "GLFW"
};

pub fn build() -> Option<Box<GLFWBackend>> {
    let mut gilrs = Gilrs::new().unwrap();
}
