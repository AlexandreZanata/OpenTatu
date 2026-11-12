# Recipe: add a dashboard page

1. Make sure the data comes from a use case in `opentatu-services`
   ([add-use-case.md](add-use-case.md)). The dashboard never touches the database.
2. Create `crates/dashboard/src/pages/<name>.rs` and declare it in `pages/mod.rs`:

   ```rust
   #[derive(Template)]
   #[template(path = "pages/<name>.html")]
   struct NamePage {
       layout: Layout,          // required by base.html
       // view data: domain types or view::* structs, already prepared
   }

   pub async fn page(State(state): State<DashboardState>) -> Result<Html<String>, PageError> {
       render(&NamePage { layout: Layout::default(), /* ... */ })
   }
   ```

3. Create `crates/dashboard/templates/pages/<name>.html`:

   ```text
   {% extends "base.html" %}
   {% import "components/ui.html" as ui %}
   {% block title %}<Título> · OpenTatu{% endblock %}
   {% block content %}
   {% call ui::card("<Título>") %} ... {% endcall %}
   {% endblock %}
   ```

   - Use only existing components ([../COMPONENTS.md](../COMPONENTS.md)); need a new one?
     [add-ui-component.md](add-ui-component.md).
   - Owner-facing text in pt-BR; dates only via `ui::datetime`.
4. Register the route in `crates/dashboard/src/routes.rs` (`router`) — the only place routes
   are registered. Add a navigation link in `templates/base.html` if it is a top-level page.
5. Forms (when needed): POST only, CSRF token, `SameSite=Strict` session cookie, and the use
   case validates everything again (docs/SECURITY.md).
6. Test in `routes.rs` with `oneshot` and a `Host` header: status, key pt-BR text, security
   headers.
7. `cargo xtask check`.
