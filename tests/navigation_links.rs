use rust_skgen::discover::navigation_links;

#[test]
fn returns_only_links_within_navigation_elements() {
    let html = r#"
        <html>
            <body>
                <header><a href="/home">Home</a></header>
                <nav>
                    <a href="/getting-started">Getting started</a>
                    <ul><li><a href="/components">Components</a></li></ul>
                </nav>
                <main><a href="/installation">Installation</a></main>
                <footer><a href="/contact">Contact</a></footer>
            </body>
        </html>
    "#;

    assert_eq!(
        navigation_links(html),
        vec!["/getting-started", "/components"]
    );
}
