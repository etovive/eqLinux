# eqLinux

Ecualizador de audio y mezclador de volumen para Linux. eqLinux crea una salida de audio virtual compatible con PulseAudio/PipeWire, procesa el sonido y lo envía a la salida que elijas.

## Funciones

- Ecualización global en modos Básico, Avanzado y Experto.
- Controles de volumen y balance, presets y ecualizador por aplicación.
- Enrutamiento de audio mediante una salida virtual de PulseAudio/PipeWire.
- Grabación de audio procesado y procesamiento de archivos WAV desde la línea de comandos.
- Interfaz en español e inglés.
- Opción en Ajustes para iniciar con Linux minimizada en la bandeja del sistema.

## Requisitos

- Linux con un servidor de audio PulseAudio o PipeWire con compatibilidad PulseAudio.
- Rust y Cargo.
- Bibliotecas de desarrollo de PulseAudio, GTK 3 y Ayatana AppIndicator.

En Debian o Ubuntu puedes instalar las dependencias de compilación con:

```bash
sudo apt update
sudo apt install build-essential pkg-config libpulse-dev libgtk-3-dev libayatana-appindicator3-dev
```

Para ejecutar el instalador también se necesita la biblioteca de ejecución de Ayatana AppIndicator:

```bash
sudo apt install libayatana-appindicator3-1
```

## Compilar y ejecutar

El instalador compila la aplicación automáticamente. Después de instalar los requisitos, basta con ejecutar:

```bash
./eqLinux-install.sh
```

También puedes compilarla y ejecutarla manualmente durante el desarrollo:

```bash
cargo build --release
cargo run --release
```

El instalador crea el acceso del menú y coloca el binario en `~/.local/bin`. Para una instalación global, edita `eqLinux-install.sh` y cambia `INSTALL_SYSTEM` a `"true"`.

## Opciones de línea de comandos

```text
eqlinux --help
eqlinux --version
eqlinux --list-sinks
eqlinux --no-route
eqlinux --minimized
eqlinux --diagnose
eqlinux --render entrada.wav salida.wav
```

`--minimized` inicia eqLinux minimizada; la casilla de Ajustes crea o quita el acceso de autoinicio de la sesión. `--no-route` abre la interfaz sin modificar el enrutamiento del audio del sistema. `--render` procesa un WAV con los ajustes guardados actualmente.

## Inspiración y agradecimientos

Gracias al equipo de [eqMac](https://github.com/bitgapp/eqMac) por su trabajo, que sirvió como referencia para eqLinux. En particular, el proyecto toma como referencia:

- Los datos de presets avanzados, portados desde `AdvancedEqualizerDefaultPresets`.
- Las diez frecuencias centrales y el ancho de banda de las bandas de ecualización.
- La curva de volumen y algunos aspectos de la presentación visual y los controles.
- El enfoque de enrutamiento a través de una salida virtual.

eqLinux es un proyecto independiente y no está afiliado con eqMac. Las atribuciones y avisos correspondientes a los componentes basados en ese trabajo se conservan en el código fuente.

## Licencia

eqLinux se distribuye bajo la licencia Apache 2.0. Consulta el archivo [LICENSE](./LICENSE) y las atribuciones conservadas en el código fuente.
