# AGENTS.md - radix-skill

## Proyecto

`radix-skill` es una CLI en Rust que extrae documentación pública de sitios web y la transforma en una skill utilizable por agentes de inteligencia artificial. La primera integración objetivo es la documentación de Radix Primitives, pero el diseñoo debe permitir incorporar otros sitios mediante adaptadores o configuración, sin acoplar el núcleo a un proveedor.

La aplicación usa Rust estable y Cargo. Separa la obtención HTTP, el descubrimiento y parseo de páginas, el modelo normalizado de documentación y la generación de archivos de skill; la CLI solo orquesta esas capas.

## Comandos

- Ejecutar: `cargo run -- <argumentos>`
- Tests: `cargo test`
- Lint/formato: `cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings`

## Estilo y convenciones

- Usar la versión estable actual de Rust y declarar la versión mínima soportada (MSRV) en `Cargo.toml` cuando el proyecto se inicialice.
- Aplicar `rustfmt`; no desactivar reglas de Clippy sin una justificación concreta en el código.
- Usar `snake_case` para funciones, modulos y variables; `PascalCase` para tipos y traits; `SCREAMING_SNAKE_CASE` para constantes.
- Escribir código, nombres, mensajes de error, documentación tecnica e interfaz de la CLI en inglés; `docs/constitution.md` y las specs en `specs/` se escriben en español.
- Propagar errores con tipos explícitos y contexto útil; no usar `unwrap` ni `expect` en rutas de ejecución normales.
- Mantener la lógica de red, parseo y escritura de archivos testeable y fuera de los handlers de la CLI.

## Reglas

- Lee `docs/constitution.md` y la spec activa en `specs/` antes de tocar código. Si no existen, indícalo y sigue las instrucciones de la tarea.
- No añadas dependencias sin actualizar antes la spec activa.
- No modifiques archivos dentro de `specs/` salvo petición explicita.
- Respeta `robots.txt`, los términos de uso aplicables, límites de peticiones y políticas de acceso de cada sitio. No implementes mecanismos para eludir autenticación, controles de acceso o rate limits.
- Identifica las peticiones de scraping con un `User-Agent` configurable y aplica timeouts, reintentos acotados y limitación de concurrencia.
- Trata todo HTML, URL y contenido remoto como no confiable. Valida URLs, limita redirecciones y evita que datos remotos determinen rutas de escritura fuera del directorio de salida elegido.
- Preserva las URL de origen y la atribución en la salida generada. No inventes contenido ni presentes como documentación información no extraída o no verificada.
- Genera skills deterministas a partir del modelo normalizado; no mezcles el parseo de un sitio concreto con el formato de salida de una skill.
- No añadas integraciones externas, telemetría, almacenamiento persistente ni soporte para proveedores de IA sin preguntar primero.
- No modifiques archivos generados, credenciales, secretos ni configuración de CI salvo que la tarea lo requiera expresamente.

## Al terminar cualquier tarea

- Ejecuta `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings` y `cargo test` cuando el proyecto sea compilable.
- Si algún comando no puede ejecutarse, informa el motivo y las verificaciones que sí se realizaron.
- Actualiza la documentación y los tests afectados por cambios de comportamiento público.
