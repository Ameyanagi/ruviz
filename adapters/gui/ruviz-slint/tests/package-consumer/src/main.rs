slint::include_modules!();

fn main() {
    let _component_type_check = Consumer::new;
    let _controller_type_check = |component: &Consumer| {
        ruviz_slint::RuvizController::attach(component)
    };
}
