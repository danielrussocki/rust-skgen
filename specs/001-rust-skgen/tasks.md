# Tareas: Generación y actualización de skills

- [x] T01. Registrar en la especificación activa la justificación de las dependencias necesarias para HTTP(S), URLs, HTML, JSON y huellas.
RF: RF-1, RF-3, RF-4, RF-6.
Hecho cuando: La especificación justifica explícitamente cada categoría de dependencia antes de modificar el manifiesto.

- [x] T02. Inicializar el proyecto Cargo con la versión mínima de Rust soportada y los metadatos básicos.
RF: RF-1 a RF-7.
Hecho cuando: `cargo check` finaliza correctamente y `Cargo.toml` declara la MSRV.

- [x] T03. Añadir las dependencias justificadas para HTTP(S), URLs, parseo HTML, JSON, huellas y argumentos de CLI.
RF: RF-1, RF-3, RF-4, RF-6.
Hecho cuando: Las dependencias añadidas coinciden con las categorías justificadas en la especificación y `cargo check` finaliza correctamente.

- [x] T04. Crear la estructura inicial de módulos y las interfaces de orquestación sin lógica de red ni almacenamiento.
RF: RF-1 a RF-7.
Hecho cuando: Los módulos `cli`, `domain`, `fetch`, `policy`, `discover`, `extract`, `render`, `metadata`, `storage` y `service` compilan con interfaces mínimas.

- [x] T05. Definir el tipo validado de nombre de skill y la validación de slug.
RF: RF-1, RF-6.
Hecho cuando: Tests deterministas aceptan slugs de hasta 64 caracteres y rechazan mayúsculas, caracteres no ASCII, guiones extremos y longitudes mayores.

- [x] T06. Definir los tipos de URL de origen, alcance, límite de sitio, modo de recorrido y formato de contenido.
RF: RF-1, RF-3, RF-4, RF-6.
Hecho cuando: Los tipos expresan todos los valores permitidos y sus valores predeterminados sin usar cadenas libres fuera de la frontera de CLI.

- [x] T07. Validar combinaciones de configuración de descubrimiento.
RF: RF-3, RF-6.
Hecho cuando: Tests cubren valores predeterminados, máximo positivo, límite de sitio solo con mismo sitio y subdominios adicionales solo con dominio base.

- [x] T08. Implementar la normalización de URLs para equivalencia de visita.
RF: RF-3.
Hecho cuando: Tests prueban que URLs que solo difieren por consulta o fragmento se consideran iguales.

- [x] T09. Implementar las reglas de pertenencia de URL para host exacto, mismo origen y dominio base.
RF: RF-3.
Hecho cuando: Tests cubren esquema, host, puerto, subdominios autorizados y el requisito de enlace previo.

- [x] T10. Implementar las reglas de alcance por prefijo de ruta y directorio padre.
RF: RF-3.
Hecho cuando: Tests deterministas aceptan únicamente URLs dentro del prefijo o directorio correspondiente.

- [x] T11. Implementar la detección de enlaces desde elementos HTML de navegación de la página inicial.
RF: RF-3.
Hecho cuando: Un documento de prueba incluye enlaces dentro y fuera de navegación, y solo se devuelven los primeros.

- [x] T12. Implementar el cliente HTTP(S) con `User-Agent`, tiempos de espera, reintentos acotados, una solicitud concurrente y redirecciones deshabilitadas.
RF: RF-1, RF-3.
Hecho cuando: Un servidor local de prueba verifica cabecera, ausencia de seguimiento de redirecciones, reintentos limitados y concurrencia máxima de una.

- [ ] T13. Implementar la lectura y evaluación de `robots.txt` antes de solicitar cada página.
RF: RF-3.
Hecho cuando: Tests locales prueban una URL permitida y otra prohibida para el `User-Agent` configurado.

- [ ] T14. Implementar la extracción de enlaces absolutos y relativos desde HTML.
RF: RF-3.
Hecho cuando: Tests convierten enlaces relativos a URLs absolutas y descartan esquemas no HTTP(S).

- [ ] T15. Implementar la extracción de contenido documental normalizado con URL de fuente.
RF: RF-3, RF-4.
Hecho cuando: Un HTML de prueba produce una página documental con contenido y URL atribuible, y contenido no documental produce un error.

- [ ] T16. Implementar el recorrido completo determinista del sitio.
RF: RF-3, RF-4.
Hecho cuando: Tests con un grafo local verifican recorrido hasta agotamiento, orden estable, inclusión obligatoria de la página inicial y eliminación de duplicados.

- [ ] T17. Implementar los recorridos de un nivel y con límite configurable.
RF: RF-3, RF-4.
Hecho cuando: Tests verifican que un nivel no encola descendientes y que el modo limitado cuenta la inicial, usa 100 por defecto y publica al alcanzar el máximo.

- [ ] T18. Integrar errores de redirección, acceso prohibido, extracción fallida y ausencia de páginas válidas como fallo atómico de una skill.
RF: RF-1, RF-2, RF-3.
Hecho cuando: Cada fallo de una página conserva intacta la skill previa y no deja una nueva skill parcial.

- [ ] T19. Definir el modelo de metadatos JSON y su serialización validada.
RF: RF-4, RF-5, RF-6.
Hecho cuando: Tests serializan y deserializan todos los campos de configuración requeridos y rechazan versiones, valores o campos esenciales inválidos.

- [ ] T20. Calcular y verificar la huella del contenido gestionado.
RF: RF-4, RF-6.
Hecho cuando: Tests detectan una modificación manual del contenido y aceptan contenido sin cambios.

- [ ] T21. Implementar el renderizado determinista del formato de guía con referencias.
RF: RF-4.
Hecho cuando: El resultado contiene objetivo e instrucciones derivados de contenido extraído, contenido ordenado y URL atribuida de cada fuente.

- [ ] T22. Implementar el renderizado determinista del formato de contenido organizado.
RF: RF-4.
Hecho cuando: El resultado contiene solo contenido extraído ordenado por página o tema y sus URLs de fuente.

- [ ] T23. Implementar la localización de skills gestionadas y la lectura de sus metadatos.
RF: RF-5, RF-6.
Hecho cuando: Tests distinguen skills gestionadas, no gestionadas, con metadatos ausentes e inválidos.

- [ ] T24. Implementar creación transaccional de una skill en la ruta de salida especificada.
RF: RF-1, RF-2, RF-4.
Hecho cuando: La creación publica contenido y metadatos completos bajo la ruta requerida y falla sin cambios si el destino ya existe.

- [ ] T25. Implementar reemplazo, renombrado, restauración y bloqueo exclusivo de una skill.
RF: RF-2, RF-5, RF-6.
Hecho cuando: Tests cubren fallo de escritura, conflicto de nombre, renombrado al mismo nombre, restauración de la versión previa y rechazo de una segunda operación simultánea.

- [ ] T26. Implementar el servicio de creación que integra validación, descubrimiento, renderizado y publicación.
RF: RF-1, RF-2, RF-3, RF-4, RF-7.
Hecho cuando: Un test de extremo a extremo local crea una skill con fuentes atribuidas y devuelve un resultado de éxito.

- [ ] T27. Implementar el servicio de actualización de una skill con cambios de configuración individuales.
RF: RF-2, RF-3, RF-4, RF-6.
Hecho cuando: Tests actualizan nombre, URL, alcance, límite, recorrido, máximo y formato, y persisten la nueva configuración.

- [ ] T28. Implementar la detección de metadatos ausentes, inválidos o con huella discrepante y el flujo de confirmación de reconstrucción.
RF: RF-5, RF-6.
Hecho cuando: Tests cubren confirmación aceptada, rechazada y sin respuesta; los dos últimos casos no modifican la skill.

- [ ] T29. Implementar la actualización de todas las skills gestionadas y de selecciones dirigidas.
RF: RF-5, RF-6, RF-7.
Hecho cuando: Tests cubren ausencia de skills, selección mixta, continuación tras fallos individuales y conservación de cada skill fallida.

- [ ] T30. Rechazar cambios de configuración cuando la actualización dirigida selecciona varias skills.
RF: RF-6, RF-7.
Hecho cuando: Un test verifica que la operación se rechaza antes de modificar cualquier skill.

- [ ] T31. Implementar el contrato de argumentos de `create` y `update`.
RF: RF-1, RF-3, RF-5, RF-6.
Hecho cuando: Tests de CLI cubren argumentos obligatorios, valores predeterminados, combinaciones inválidas y parámetros de actualización individual o múltiple.

- [ ] T32. Implementar mensajes en inglés, resultados por skill y códigos de salida.
RF: RF-7.
Hecho cuando: Tests de CLI verifican salida estándar, salida de error y códigos `0`, `1` y `2` para éxito, fallo de skill y argumentos inválidos.

- [ ] T33. Ejecutar la suite completa y corregir incumplimientos de formato, lint y pruebas.
RF: RF-1 a RF-7.
Hecho cuando: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings` y `cargo test` finalizan correctamente.
