use minijinja::Environment;

pub fn build_env() -> Environment<'static> {
    let mut env = Environment::new();
    env.add_template("_base.html", include_str!("../templates/_base.html")).unwrap();
    env.add_template("dashboard.html", include_str!("../templates/dashboard.html")).unwrap();
    env.add_template("settings.html", include_str!("../templates/settings.html")).unwrap();
    env
}
