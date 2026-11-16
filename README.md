# Forever Fishing

Asistente de pesca de consola para Windows x64, escrito en Rust. Escucha únicamente el proceso de WoW y sus procesos hijos mediante WASAPI process loopback (no el micrófono ni la mezcla de todo el PC), reconoce el inicio de una muestra de picada y pulsa Espacio para recoger. Tras soltar la tecla espera entre 1000 y 2000 ms y vuelve a pulsarla para lanzar.

## Preparación

Configura **Espacio** en el juego/addon para recoger y volver a lanzar con la misma tecla. El programa no configura esos bindings. Lanza manualmente la primera vez. Activa el saqueo automático si tu configuración lo necesita.

El ejecutable integra las tres variantes modernas OGG de [Fishing Hooked (3355)](https://www.wowhead.com/sound=3355/fishing-hooked) aportadas por el usuario y la referencia histórica **FishBite.ogg** de Wowhead Classic. No necesita descargas ni archivos externos. Cada variante tiene su propia firma y duración; si varias coinciden en un bloque se usa la de mayor similitud.

| Variante | Archivo completo | Región activa usada |
| --- | ---: | ---: |
| FishBite (Classic) | 2171 ms | 1608 ms |
| ver2_1 | 3492 ms | 2229 ms |
| ver2_2 | 3163 ms | 2099 ms |
| ver2_3 | 3030 ms | 1590 ms |

La región activa excluye el silencio y la cola por debajo del 8 % del pico. No conviene usar toda la duración del archivo como ventana de pulsación. Créditos de los sonidos en `THIRD_PARTY.md`.

## Uso

Prueba primero sin enviar teclas:

```powershell
.\forever-fishing.exe run --dry-run
```

Vuelve al juego y pulsa **F8** para activar. Escucha y compara las detecciones de consola con las picadas reales. Después:

```powershell
.\forever-fishing.exe run
```

- `--recast-min-ms 1000 --recast-max-ms 2000` configura la espera aleatoria entre recoger y lanzar (milisegundos; mínimo ≤ máximo; límites 0–30000).
- `--diagnostics` muestra la similitud máxima por segundo y las detecciones aceptadas. Consulta el [inventario y análisis de sonidos](docs/sound-research.md).
- **F8** activa/pausa; **Escape** termina. Empieza pausado.
- Al perder el foco del juego se pausa y cancela acciones pendientes. Pulsa F8 en el juego para reanudar.
- Por defecto exige que el título de la ventana activa contenga `World of Warcraft`. Puedes cambiarlo con `--window "título del cliente"`.
- `--threshold 0.96` ajusta la similitud mínima (0.5–1). Subirlo reduce sensibilidad; bajarlo aumenta falsos positivos.
- Abre WoW antes de iniciar el asistente. Se selecciona el proceso por el título de su ventana; si hay varias instancias, usa `--process-id PID`. Si WoW se cierra, el asistente termina y debes reiniciarlo con el juego.

## Detección y tiempos

Compara seis ventanas espectrales FFT de 20 ms en 20 bandas, con diez alineaciones posibles del inicio de la muestra con el audio entrante, normalizadas para tolerar cambios de volumen. No espera a escuchar el sonido completo: tras los primeros 120 ms elige un retardo aleatorio dentro de la duración restante estimada, con margen para latencia. No puede seleccionar instantes anteriores a la detección ni garantizar que una muestra diferente dure lo mismo. Las pulsaciones duran 20 ms; el intervalo posterior incluye unos pocos milisegundos de procesamiento. La aleatoriedad no garantiza detección perfecta ni cambios en cómo el juego interpreta las teclas.

Descarta audio atrasado y acciones vencidas, e ignora detecciones mientras recoge/relanza y durante dos segundos tras relanzar. Una picada perdida no provoca relanzamientos automáticos: pausa, lanza manualmente y reanuda. La captura predeterminada excluye otras aplicaciones, pero incluye música, ambiente y voces del propio juego. Puedes reducirlos en WoW para mejorar la detección. Las pulsaciones requieren que la ventana activa pertenezca al proceso seleccionado y tenga el título esperado.

Las pruebas reproducen las cuatro muestras con veinte desfases temporales y volumen al 20 %, y comprueban rechazo de silencio, otro tono y ruido blanco. La peor similitud observada es 0.9961; la detección ocurre antes de 180 ms desde el inicio de la región activa. **Pendiente de prueba dentro de WoW Forever**, con su mezcla de audio real.

## Audio exclusivo de WoW y EarTrumpet

No requiere configurar EarTrumpet, cables virtuales ni cambiar el dispositivo de salida. Windows captura los flujos del proceso seleccionado y sus hijos, incluso si usa una salida distinta de la predeterminada. No separa efectos, música y ambiente dentro de WoW.

Requiere Windows build 20348 o posterior (incluido Windows 11). Si falla la selección o activación, se muestra un error; nunca se cambia automáticamente a capturar todo el PC. La captura por dispositivo anterior sigue disponible explícitamente con `--system-audio`. Estas opciones funcionan tanto en `run` como en `record`:

```powershell
.\forever-fishing.exe run --diagnostics
.\forever-fishing.exe run --process-id 12345 --diagnostics
.\forever-fishing.exe record captura.wav --seconds 5 --process-id 12345
.\forever-fishing.exe run --system-audio
```

El PID cambia al reiniciar el juego; omite `--process-id` para selección automática por título. No selecciones el launcher ni el proceso de voz. Si hay varias ventanas del mismo proceso se consideran una única instancia. Para grabar un cliente con otro título, indica su PID.

Referencia: [captura loopback por aplicación de Microsoft](https://learn.microsoft.com/en-us/samples/microsoft/windows-classic-samples/applicationloopbackaudio-sample/).

## Referencias personalizadas

Puedes sustituir las muestras integradas por uno o varios archivos WAV u OGG Vorbis:

```powershell
.\forever-fishing.exe run picada1.ogg picada2.ogg picada3.wav --dry-run
.\forever-fishing.exe record captura.wav --seconds 5
```

Para crear una referencia, recorta una sola picada completa de la grabación, sin lanzamiento, música ni otros efectos. Se admiten archivos de hasta 30 segundos con región activa de 250 ms a 10 s. La conversión a mono/16 kHz es interna; no necesita FFmpeg.

## Desarrollo y releases

```powershell
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --locked --release
```

Rust 1.99.1, Windows. Ejecutable en `target/release/forever-fishing.exe`.

Se utiliza **release-please**, siguiendo la configuración de `Mancuerna/ndpiviewer-rs`: Conventional Commits (`feat:`, `fix:`...), PR de versión/changelog, release en borrador y publicación solo tras compilar y probar. El workflow adjunta un ZIP Windows x64 con ejecutable, README y licencia, además de SHA-256. Activa en GitHub Settings → Actions → General la opción que permite a GitHub Actions crear pull requests. No requiere un PAT. `workflow_dispatch` permite reintentar una versión cuyo tag ya exista y coincida con Cargo.toml.

Referencias técnicas: [WASAPI Rust](https://github.com/HEnquist/wasapi-rs), [release-please de referencia](https://github.com/Mancuerna/ndpiviewer-rs/blob/main/.github/workflows/release.yml).
