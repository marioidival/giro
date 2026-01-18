use askama::Template;

#[derive(Template)]
#[template(path = "base.html")]
pub struct BaseTemplate {
    pub logged_in: bool,
}

fn main() {
    println!("Testing base template with logged_in = true:");
    let template = BaseTemplate { logged_in: true };
    match template.render() {
        Ok(html) => {
            assert!(
                html.contains("<!DOCTYPE html>"),
                "Should have HTML5 doctype"
            );
            assert!(
                html.contains("tailwindcss.com"),
                "Should include Tailwind CDN"
            );
            assert!(html.contains("htmx.org"), "Should include HTMX CDN");
            assert!(html.contains("csrf-token"), "Should include CSRF meta tag");
            assert!(html.contains("Home"), "Should include navigation");
            assert!(html.contains("Loops"), "Should include navigation");
            assert!(
                html.contains("Logout"),
                "Should include logout when logged in"
            );
            println!("✅ All assertions passed for logged_in = true");
        }
        Err(e) => {
            eprintln!("❌ Template rendering failed: {}", e);
            std::process::exit(1);
        }
    }

    println!("\nTesting base template with logged_in = false:");
    let template = BaseTemplate { logged_in: false };
    match template.render() {
        Ok(html) => {
            assert!(
                !html.contains("<nav"),
                "Should not show navigation when not logged in"
            );
            assert!(
                html.contains("tailwindcss.com"),
                "Should still include Tailwind CDN"
            );
            assert!(html.contains("htmx.org"), "Should still include HTMX CDN");
            assert!(
                html.contains("csrf-token"),
                "Should still include CSRF meta tag"
            );
            println!("✅ All assertions passed for logged_in = false");
        }
        Err(e) => {
            eprintln!("❌ Template rendering failed: {}", e);
            std::process::exit(1);
        }
    }

    println!("\n✅ All template tests passed successfully!");
}
