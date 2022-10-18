mod state;
mod texture;
mod window;
mod framework;

pub fn run(){
  let fw = pollster::block_on(framework::Setup::setup::<state::State>("test"));
  framework::Setup::start::<state::State>(fw, state::Setup {});
}