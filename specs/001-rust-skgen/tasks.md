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

- [x] T13. Implementar la lectura y evaluación de `robots.txt` antes de solicitar cada página.
RF: RF-3.
Hecho cuando: Tests locales prueban una URL permitida y otra prohibida para el `User-Agent` configurado.

- [x] T14. Implementar la extracción de enlaces absolutos y relativos desde HTML.
RF: RF-3.
Hecho cuando: Tests convierten enlaces relativos a URLs absolutas y descartan esquemas no HTTP(S).

- [x] T15. Implementar la extracción de contenido documental normalizado con URL de fuente.
RF: RF-3, RF-4.
Hecho cuando: Un HTML de prueba produce una página documental con contenido y URL atribuible, y contenido no documental produce un error.

- [x] T16. Implementar el recorrido completo determinista del sitio.
RF: RF-3, RF-4.
Hecho cuando: Tests con un grafo local verifican recorrido hasta agotamiento, orden estable, inclusión obligatoria de la página inicial y eliminación de duplicados.

- [x] T17. Implementar los recorridos de un nivel y con límite configurable.
RF: RF-3, RF-4.
Hecho cuando: Tests verifican que un nivel no encola descendientes y que el modo limitado cuenta la inicial, usa 100 por defecto y publica al alcanzar el máximo.

- [x] T18. Integrar errores de redirección, acceso prohibido, extracción fallida y ausencia de páginas válidas como fallo atómico de una skill.
RF: RF-1, RF-2, RF-3.
Hecho cuando: Cada fallo de una página conserva intacta la skill previa y no deja una nueva skill parcial.

- [x] T19. Definir el modelo de metadatos JSON y su serialización validada.
RF: RF-4, RF-5, RF-6.
Hecho cuando: Tests serializan y deserializan todos los campos de configuración requeridos y rechazan versiones, valores o campos esenciales inválidos.

- [x] T20. Calcular y verificar la huella del contenido gestionado.
RF: RF-4, RF-6.
Hecho cuando: Tests detectan una modificación manual del contenido y aceptan contenido sin cambios.

- [x] T21. Implementar el renderizado determinista del formato de guía con referencias.
RF: RF-4.
Hecho cuando: El resultado contiene objetivo e instrucciones derivados de contenido extraído, contenido ordenado y URL atribuida de cada fuente.

- [x] T22. Implementar el renderizado determinista del formato de contenido organizado.
RF: RF-4.
Hecho cuando: El resultado contiene solo contenido extraído ordenado por página o tema y sus URLs de fuente.

- [x] T23. Implementar la localización de skills gestionadas y la lectura de sus metadatos.
RF: RF-5, RF-6.
Hecho cuando: Tests distinguen skills gestionadas, no gestionadas, con metadatos ausentes e inválidos.

- [x] T24. Implementar creación transaccional de una skill en la ruta de salida especificada.
RF: RF-1, RF-2, RF-4.
Hecho cuando: La creación publica contenido y metadatos completos bajo la ruta requerida y falla sin cambios si el destino ya existe.

- [x] T25. Implementar reemplazo, renombrado, restauración y bloqueo exclusivo de una skill.
RF: RF-2, RF-5, RF-6.
Hecho cuando: Tests cubren fallo de escritura, conflicto de nombre, renombrado al mismo nombre, restauración de la versión previa y rechazo de una segunda operación simultánea.

- [x] T26. Implementar el servicio de creación que integra validación, descubrimiento, renderizado y publicación.
RF: RF-1, RF-2, RF-3, RF-4, RF-7.
Hecho cuando: Un test de extremo a extremo local crea una skill con fuentes atribuidas y devuelve un resultado de éxito.

- [x] T27. Implementar el servicio de actualización de una skill con cambios de configuración individuales.
RF: RF-2, RF-3, RF-4, RF-6.
Hecho cuando: Tests actualizan nombre, URL, alcance, límite, recorrido, máximo y formato, y persisten la nueva configuración.

- [x] T28. Implementar la detección de metadatos ausentes, inválidos o con huella discrepante y el flujo de confirmación de reconstrucción.
RF: RF-5, RF-6.
Hecho cuando: Tests cubren confirmación aceptada, rechazada y sin respuesta; los dos últimos casos no modifican la skill.

- [x] T29. Implementar la actualización de todas las skills gestionadas y de selecciones dirigidas.
RF: RF-5, RF-6, RF-7.
Hecho cuando: Tests cubren ausencia de skills, selección mixta, continuación tras fallos individuales y conservación de cada skill fallida.

- [x] T30. Rechazar cambios de configuración cuando la actualización dirigida selecciona varias skills.
RF: RF-6, RF-7.
Hecho cuando: Un test verifica que la operación se rechaza antes de modificar cualquier skill.

- [x] T31. Implementar el contrato de argumentos de `create` y `update`.
RF: RF-1, RF-3, RF-5, RF-6.
Hecho cuando: Tests de CLI cubren argumentos obligatorios, valores predeterminados, combinaciones inválidas y parámetros de actualización individual o múltiple.

- [x] T32. Implementar mensajes en inglés, resultados por skill y códigos de salida.
RF: RF-7.
Hecho cuando: Tests de CLI verifican salida estándar, salida de error y códigos `0`, `1` y `2` para éxito, fallo de skill y argumentos inválidos.

- [x] T33. Ejecutar la suite completa y corregir incumplimientos de formato, lint y pruebas.
RF: RF-1 a RF-7.
Hecho cuando: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings` y `cargo test` finalizan correctamente.

- [x] T34. Reforzar la validación de URL de origen para exigir un host HTTP(S) válido.
RF: RF-1.
Hecho cuando: Tests deterministas rechazan URLs HTTP(S) sin host y aceptan URLs HTTP(S) con host válido.

- [x] T35. Restringir los alcances de prefijo de ruta y directorio padre al esquema, host y puerto de la URL de origen.
RF: RF-3.
Hecho cuando: Tests deterministas rechazan para ambos alcances una URL de otro host, esquema o puerto aunque su ruta coincida.

- [x] T36. Incorporar una política de acceso aplicable, además de `robots.txt`, en el descubrimiento.
RF: RF-3.
Hecho cuando: Un doble de política que prohíbe una URL hace fallar la operación sin publicar una skill nueva ni alterar una existente.

- [x] T37. Rechazar respuestas que no sean HTML apto para extracción documental.
RF: RF-1, RF-3.
Hecho cuando: Un servidor local que devuelve una respuesta 2xx con contenido no HTML provoca un fallo atómico de creación y actualización.

- [x] T38. Derivar de forma determinista el objetivo y las instrucciones de la guía exclusivamente de las páginas extraídas.
RF: RF-4.
Hecho cuando: Un test de renderizado verifica que `Objective` e `Instructions` no están vacíos, proceden de contenido de prueba extraído y no incluyen texto ajeno a las fuentes.

- [x] T39. Validar estructuralmente la huella `content_digest` de los metadatos.
RF: RF-4, RF-6.
Hecho cuando: Tests de deserialización rechazan huellas vacías, prefijos distintos de `sha256:` y valores que no contienen exactamente 64 dígitos hexadecimales.

- [x] T40. Permitir actualizar una skill con metadatos válidos aunque su contenido gestionado haya sido editado manualmente.
RF: RF-5, RF-6.
Hecho cuando: Un test de servicio modifica manualmente una skill con metadatos válidos y verifica que una actualización correcta reemplaza el contenido y conserva metadatos válidos.

- [x] T41. Definir y probar la reconstrucción confirmada de metadatos ausentes o inválidos.
RF: RF-6.
Hecho cuando: Tests deterministas especifican la configuración usada al aceptar la reconstrucción y verifican que el rechazo o la ausencia de respuesta no modifica la skill.

- [x] T42. Implementar la reconstrucción confirmada de metadatos ausentes o inválidos según la configuración definida.
RF: RF-6.
Hecho cuando: Un test de servicio confirma la reconstrucción para metadatos ausentes y otro para metadatos inválidos; ambos actualizan la skill y escriben metadatos válidos.

- [x] T43. Restaurar la versión previa si falla cualquier fase de reemplazo antes de confirmar la actualización.
RF: RF-2, RF-7.
Hecho cuando: Un doble de almacenamiento que falla al limpiar el backup verifica que la operación no informa un fallo después de publicar contenido nuevo, o que restaura íntegramente la versión previa.

- [x] T44. Añadir al resultado de CLI el fallo individual de creación y su código de salida.
RF: RF-7.
Hecho cuando: Tests de salida verifican que un fallo de creación produce `Failed skill: <name>: <reason>` en error estándar y código 1.

- [x] T45. Crear el ejecutable Cargo y el bootstrap que compone CLI, servicios y almacenamiento desde el directorio actual.
RF: RF-1, RF-5, RF-6, RF-7.
Hecho cuando: `cargo run -- --help` muestra los comandos `create` y `update`, y un test de integración puede ejecutar el binario.

- [x] T46. Resolver el directorio de skills como `.agents/skills` relativo al directorio de trabajo del proceso.
RF: RF-1.
Hecho cuando: Un test de integración ejecuta `create` en un directorio temporal y verifica que la skill se publica únicamente bajo `.agents/skills/<skill-name>`.

- [x] T47. Conectar el comando `create` del ejecutable al servicio y a los resultados de salida.
RF: RF-1, RF-2, RF-3, RF-4, RF-7.
Hecho cuando: Tests de integración locales verifican creación exitosa con fuentes atribuidas, rechazo de destino existente y ausencia de directorios parciales ante un fallo de descubrimiento.

- [x] T48. Conectar el comando `update` del ejecutable a actualizaciones sin argumentos, dirigidas y con cambios individuales.
RF: RF-5, RF-6, RF-7.
Hecho cuando: Tests de integración ejecutan actualizaciones sin nombres, selecciones mixtas y una actualización individual configurada, verificando los resultados por skill y la persistencia de configuración.

- [x] T49. Implementar la confirmación interactiva de reconstrucción de metadatos en el ejecutable.
RF: RF-6.
Hecho cuando: Tests de integración con entrada simulada verifican que `y` reconstruye y actualiza, mientras que `n` y EOF no modifican la skill.

- [x] T50. Verificar de extremo a extremo los códigos de salida y los canales de salida del ejecutable.
RF: RF-7.
Hecho cuando: Tests de integración verifican código 0 para éxito, 1 para uno o más fallos de skill y 2 para argumentos inválidos, con los mensajes en los canales especificados.

- [x] T51. Ejecutar la matriz completa de pruebas de requisitos y cerrar los huecos de cobertura restantes.
RF: RF-1 a RF-7.
Hecho cuando: Cada requisito funcional y caso límite de la especificación tiene una prueba determinista identificable y `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings` y `cargo test` finalizan correctamente.

- [x] T52. Añadir pruebas de regresión que exijan confirmación cuando la huella del contenido de una skill gestionada no coincide con sus metadatos.
RF: RF-6.
Hecho cuando: Tests deterministas verifican que una huella discrepante solicita confirmación, que `n` y EOF no modifican la skill, y que `y` la reconstruye correctamente.

- [x] T53. Detectar la huella discrepante al cargar una skill gestionada para actualizarla.
RF: RF-6.
Hecho cuando: La actualización compara `SKILL.md` con `content_digest` antes de descubrir páginas y ejecuta el flujo de confirmación de T52 si no coinciden.

- [x] T54. Añadir una prueba determinista de recuperación tras una interrupción entre el respaldo de la skill previa y la publicación de la sustitución.
RF: RF-2.
Hecho cuando: La prueba simula una interrupción después de mover la skill al backup y verifica que al reabrir el almacenamiento la versión previa sigue disponible en su ruta pública sin cambios.

- [x] T55. Implementar la recuperación de respaldos pendientes antes de crear o reemplazar una skill.
RF: RF-2.
Hecho cuando: La recuperación de T54 restaura atómicamente el backup pendiente cuando no existe una publicación válida y no altera una skill publicada correctamente.

- [x] T56. Definir una política de condiciones de acceso aplicables que pueda componerse con `robots.txt` sin interpretar ni eludir controles remotos.
RF: RF-3.
Hecho cuando: La política tiene una implementación concreta y testeable que permite o prohíbe URLs según condiciones de acceso explícitamente configuradas.

- [x] T57. Conectar la política combinada de `robots.txt` y condiciones de acceso en los comandos `create` y `update`.
RF: RF-3.
Hecho cuando: Tests de integración verifican que una URL prohibida por condiciones de acceso hace fallar creación y actualización sin publicar contenido parcial.

- [x] T58. Añadir pruebas de integración para los fallos de URL inicial de creación.
RF: RF-1, RF-2, RF-3.
Hecho cuando: Un servidor local verifica que URL con esquema no HTTP(S), URL inicial redirigida y URL inicial inaccesible devuelven fallo y no crean directorios parciales.

- [x] T59. Añadir una prueba de integración para un lote de actualizaciones con un fallo de descubrimiento y una skill correcta.
RF: RF-2, RF-7.
Hecho cuando: El ejecutable conserva la skill fallida, actualiza la correcta, informa ambos resultados y finaliza con código 1.

- [ ] T60. Completar la matriz de trazabilidad de RF-1 a RF-7 y casos límite con pruebas deterministas identificables.
RF: RF-1 a RF-7.
Hecho cuando: Cada cláusula funcional y caso límite de la spec referencia al menos un test que se ejecuta correctamente mediante `cargo test`.

- [ ] T61. Ejecutar las verificaciones finales de formato, lint y pruebas tras cerrar las tareas pendientes.
RF: RF-1 a RF-7.
Hecho cuando: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings` y `cargo test` finalizan correctamente.
