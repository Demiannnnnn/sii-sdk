//! Command line argument parser definitions for the `sii` binary.

use clap::{Args, Parser, Subcommand, ValueEnum};
use sii::codes::Ambiente;
use std::path::PathBuf;

/// Command line interface for Chilean SII Electronic Tax Documents (DTE).
#[derive(Parser, Debug)]
#[command(
    name = "sii",
    about = "Herramienta de línea de comandos para Documentos Tributarios Electrónicos (DTE) del SII de Chile",
    version
)]
pub(crate) struct Cli {
    /// Emite la salida en formato JSON estructurado con claves en inglés.
    #[arg(long, global = true)]
    pub(crate) json: bool,

    /// Modo silencioso (suprime mensajes informativos no esenciales).
    #[arg(short, long, global = true)]
    pub(crate) quiet: bool,

    /// Comando principal a ejecutar.
    #[command(subcommand)]
    pub(crate) command: Option<Commands>,
}

/// Primary top-level subcommands.
#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    /// Operaciones sobre Documentos Tributarios Electrónicos individuales (DTE).
    Dte(Box<DteArgs>),
    /// Operaciones sobre sobres de envío al SII y consulta de estado.
    Envio(EnvioArgs),
    /// Operaciones sobre libros de compra y venta electrónicos (IECV).
    Libro(LibroArgs),
}

/// Target execution environment for SII operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum AmbienteArg {
    /// Ambiente de certificación y pruebas (maullin).
    #[value(name = "certificacion", alias = "cert", alias = "maullin")]
    Certificacion,
    /// Ambiente real de producción (palena).
    #[value(name = "produccion", alias = "prod", alias = "palena")]
    Produccion,
}

impl From<AmbienteArg> for Ambiente {
    fn from(arg: AmbienteArg) -> Self {
        match arg {
            AmbienteArg::Certificacion => Self::Certificacion,
            AmbienteArg::Produccion => Self::Produccion,
        }
    }
}

// ---------------------------------------------------------------------------
// DTE Subcommands
// ---------------------------------------------------------------------------

/// Subcommands under `sii dte`.
#[derive(Args, Debug)]
pub(crate) struct DteArgs {
    /// Subcomando de DTE a ejecutar.
    #[command(subcommand)]
    pub(crate) command: DteCommands,
}

/// Commands for working with single electronic tax documents.
#[derive(Subcommand, Debug)]
pub(crate) enum DteCommands {
    /// Construye un DTE sin timbrar a partir de un archivo JSON o de argumentos CLI.
    Build(Box<DteBuildArgs>),
    /// Aplica un timbre electrónico (TED) con CAF a un DTE.
    Timbrar(DteTimbrarArgs),
    /// Firma digitalmente un DTE timbrado con `XMLDSig` utilizando un certificado digital.
    Firmar(DteFirmarArgs),
}

/// Arguments for `sii dte build`.
#[derive(Args, Debug)]
pub(crate) struct DteBuildArgs {
    /// Ruta a archivo JSON con la definición del DTE.
    #[arg(short, long)]
    pub(crate) input: Option<PathBuf>,

    /// Código de tipo de DTE (33: Factura Afecta, 34: Factura Exenta, 39: Boleta, 41: Boleta Exenta, 52: Guía, 56: ND, 61: NC).
    #[arg(long)]
    pub(crate) tipo: Option<u16>,

    /// Folio del documento (opcional; default 1 o el asignado al timbrar).
    #[arg(long)]
    pub(crate) folio: Option<u64>,

    /// Fecha de emisión del documento (formato YYYY-MM-DD; default: fecha actual).
    #[arg(long)]
    pub(crate) fecha: Option<String>,

    /// RUT del contribuyente emisor (ej. 76.086.428-5).
    #[arg(long)]
    pub(crate) emisor_rut: Option<String>,

    /// Razón social del contribuyente emisor.
    #[arg(long)]
    pub(crate) emisor_razon: Option<String>,

    /// Giro comercial del emisor.
    #[arg(long)]
    pub(crate) emisor_giro: Option<String>,

    /// Código de actividad económica del emisor (ej. 465900).
    #[arg(long)]
    pub(crate) emisor_acteco: Option<u32>,

    /// Dirección del emisor.
    #[arg(long)]
    pub(crate) emisor_dir: Option<String>,

    /// Comuna del emisor.
    #[arg(long)]
    pub(crate) emisor_comuna: Option<String>,

    /// Ciudad de origen del emisor (opcional).
    #[arg(long)]
    pub(crate) emisor_ciudad: Option<String>,

    /// RUT del receptor (ej. 77.777.777-7).
    #[arg(long)]
    pub(crate) receptor_rut: Option<String>,

    /// Razón social del receptor.
    #[arg(long)]
    pub(crate) receptor_razon: Option<String>,

    /// Giro comercial del receptor (opcional).
    #[arg(long)]
    pub(crate) receptor_giro: Option<String>,

    /// Dirección del receptor (opcional).
    #[arg(long)]
    pub(crate) receptor_dir: Option<String>,

    /// Comuna del receptor (opcional).
    #[arg(long)]
    pub(crate) receptor_comuna: Option<String>,

    /// Ciudad del receptor (opcional).
    #[arg(long)]
    pub(crate) receptor_ciudad: Option<String>,

    /// Nombre o descripción del primer ítem/producto.
    #[arg(long)]
    pub(crate) item_nombre: Option<String>,

    /// Precio unitario o monto neto del ítem en pesos enteros.
    #[arg(long)]
    pub(crate) item_precio: Option<i64>,

    /// Cantidad del ítem (default: 1).
    #[arg(long)]
    pub(crate) item_cantidad: Option<u32>,

    /// Código de tipo de documento referenciado (obligatorio para Notas de Débito 56 y Crédito 61).
    #[arg(long)]
    pub(crate) ref_tipo: Option<String>,

    /// Folio del documento referenciado.
    #[arg(long)]
    pub(crate) ref_folio: Option<String>,

    /// Fecha del documento referenciado (YYYY-MM-DD).
    #[arg(long)]
    pub(crate) ref_fecha: Option<String>,

    /// Código de acción de la referencia (1: Anula, 2: Corrige texto, 3: Corrige montos).
    #[arg(long)]
    pub(crate) ref_codigo: Option<u8>,

    /// Razón o motivo de la referencia.
    #[arg(long)]
    pub(crate) ref_razon: Option<String>,

    /// Archivo de salida donde guardar el resultado. Si se omite, se imprime a stdout.
    #[arg(short, long)]
    pub(crate) output: Option<PathBuf>,

    /// Formato de salida del DTE (`xml` o `json`; default: `xml`).
    #[arg(long, default_value = "xml")]
    pub(crate) format: String,
}

/// Arguments for `sii dte timbrar`.
#[derive(Args, Debug)]
pub(crate) struct DteTimbrarArgs {
    /// Archivo con el DTE a timbrar (formato JSON o XML sin timbrar).
    #[arg(short, long)]
    pub(crate) dte: PathBuf,

    /// Archivo XML con la autorización de folios (CAF) otorgada por el SII.
    #[arg(short, long)]
    pub(crate) caf: PathBuf,

    /// Folio específico a asignar en el timbre (opcional; si se omite, se usa el del DTE o el mínimo del CAF).
    #[arg(short, long)]
    pub(crate) folio: Option<u64>,

    /// Marca temporal para el timbre en formato ISO-8601 (opcional; default: hora actual).
    #[arg(long)]
    pub(crate) timestamp: Option<String>,

    /// Archivo de salida para el DTE timbrado en XML. Si se omite, se imprime a stdout.
    #[arg(short, long)]
    pub(crate) output: Option<PathBuf>,
}

/// Arguments for `sii dte firmar`.
#[derive(Args, Debug)]
pub(crate) struct DteFirmarArgs {
    /// Archivo con el DTE timbrado en XML a firmar.
    #[arg(short, long)]
    pub(crate) dte: PathBuf,

    /// Archivo con el certificado digital del emisor (`.p12`, `.pfx` o `.pem`).
    /// La contraseña se lee de la variable de entorno `SII_CERT_PASSWORD`.
    #[arg(short, long)]
    pub(crate) cert: PathBuf,

    /// Archivo con la llave privada RSA si se usa un certificado en formato PEM separado.
    #[arg(long)]
    pub(crate) cert_key: Option<PathBuf>,

    /// Ambiente de destino para la firma digital (determina algoritmo RSA-SHA1 o RSA-SHA256).
    #[arg(long)]
    pub(crate) ambiente: AmbienteArg,

    /// Archivo de salida para el DTE firmado en XML. Si se omite, se imprime a stdout.
    #[arg(short, long)]
    pub(crate) output: Option<PathBuf>,
}

// ---------------------------------------------------------------------------
// Envio Subcommands
// ---------------------------------------------------------------------------

/// Subcommands under `sii envio`.
#[derive(Args, Debug)]
pub(crate) struct EnvioArgs {
    /// Subcomando de envío a ejecutar.
    #[command(subcommand)]
    pub(crate) command: EnvioCommands,
}

/// Commands for envelope uploading and status inquiries.
#[derive(Subcommand, Debug)]
pub(crate) enum EnvioCommands {
    /// Arma un sobre de envío con DTEs firmados y lo sube al simulador o al SII.
    Enviar(EnvioEnviarArgs),
    /// Consulta el estado de procesamiento de un envío mediante su Track ID.
    Estado(EnvioEstadoArgs),
}

/// Arguments for `sii envio enviar`.
#[derive(Args, Debug)]
pub(crate) struct EnvioEnviarArgs {
    /// Uno o más archivos DTE firmados en XML a empaquetar en el sobre.
    #[arg(long)]
    pub(crate) dte: Vec<PathBuf>,

    /// Archivo XML con un sobre de envío (`EnvioDTE` o `EnvioBOLETA`) ya firmado.
    #[arg(long)]
    pub(crate) envio: Option<PathBuf>,

    /// RUT de la empresa emisora de los documentos (opcional; si se omite, se deduce del DTE).
    #[arg(long)]
    pub(crate) rut_emisor: Option<String>,

    /// RUT del usuario o entidad que realiza el envío (default: igual a `rut_emisor`).
    #[arg(long)]
    pub(crate) rut_envia: Option<String>,

    /// Fecha de resolución autorizatoria del SII (YYYY-MM-DD; default: 2026-01-01).
    #[arg(long)]
    pub(crate) fch_resol: Option<String>,

    /// Número de resolución autorizatoria del SII (default: 0 para certificación).
    #[arg(long, default_value = "0")]
    pub(crate) nro_resol: u32,

    /// Archivo con el certificado digital (`.p12`, `.pfx` o `.pem`) para autenticación y firma del sobre.
    /// La contraseña se lee de `SII_CERT_PASSWORD`.
    #[arg(short, long)]
    pub(crate) cert: Option<PathBuf>,

    /// Archivo con la llave privada en formato PEM si el certificado no la incluye.
    #[arg(long)]
    pub(crate) cert_key: Option<PathBuf>,

    /// Token de sesión activo del SII (evita solicitar semilla y token si ya se posee uno).
    #[arg(long)]
    pub(crate) token: Option<String>,

    /// Ambiente de destino (**obligatorio**; certificacion o produccion).
    #[arg(long)]
    pub(crate) ambiente: AmbienteArg,

    /// Utiliza el simulador local embebido (en memoria) para la transmisión.
    #[arg(long)]
    pub(crate) sim: bool,

    /// URL base del simulador local HTTP (ej. `<http://127.0.0.1:8080>`).
    #[arg(long)]
    pub(crate) sim_url: Option<String>,

    /// Archivo donde guardar el XML del sobre enviado (opcional).
    #[arg(short, long)]
    pub(crate) output: Option<PathBuf>,
}

/// Arguments for `sii envio estado <trackid>`.
#[derive(Args, Debug)]
pub(crate) struct EnvioEstadoArgs {
    /// Identificador numérico de seguimiento asignado por el SII (Track ID).
    pub(crate) track_id: String,

    /// RUT de la empresa contribuyente titular del envío (ej. 76.086.428-5).
    #[arg(long)]
    pub(crate) rut_emisor: Option<String>,

    /// Archivo con el certificado digital para autenticarse y obtener un token si no se suministra `--token`.
    #[arg(short, long)]
    pub(crate) cert: Option<PathBuf>,

    /// Archivo con la llave privada en formato PEM si el certificado no la incluye.
    #[arg(long)]
    pub(crate) cert_key: Option<PathBuf>,

    /// Token de sesión del SII ya adquirido previamente.
    #[arg(long)]
    pub(crate) token: Option<String>,

    /// Ambiente de destino (**obligatorio**; certificacion o produccion).
    #[arg(long)]
    pub(crate) ambiente: AmbienteArg,

    /// Consulta contra el simulador local embebido (en memoria).
    #[arg(long)]
    pub(crate) sim: bool,

    /// URL base del simulador local HTTP (ej. `<http://127.0.0.1:8080>`).
    #[arg(long)]
    pub(crate) sim_url: Option<String>,
}

// ---------------------------------------------------------------------------
// Libro Subcommands
// ---------------------------------------------------------------------------

/// Subcommands under `sii libro`.
#[derive(Args, Debug)]
pub(crate) struct LibroArgs {
    /// Subcomando de libros a ejecutar.
    #[command(subcommand)]
    pub(crate) command: LibroCommands,
}

/// Commands for purchase and sales electronic ledgers.
#[derive(Subcommand, Debug)]
pub(crate) enum LibroCommands {
    /// Construye y opcionalmente firma un Libro de Compra y Venta (IECV).
    Build(LibroBuildArgs),
}

/// Arguments for `sii libro build`.
#[derive(Args, Debug)]
pub(crate) struct LibroBuildArgs {
    /// Archivo JSON con la definición del libro y sus documentos.
    #[arg(short, long)]
    pub(crate) input: Option<PathBuf>,

    /// RUT del contribuyente emisor titular del libro.
    #[arg(long)]
    pub(crate) rut_emisor: Option<String>,

    /// RUT de quien envía el libro (default: igual a `rut_emisor`).
    #[arg(long)]
    pub(crate) rut_envia: Option<String>,

    /// Período tributario en formato YYYY-MM (ej. 2026-10).
    #[arg(long)]
    pub(crate) periodo: Option<String>,

    /// Fecha de resolución autorizatoria (YYYY-MM-DD; default: 2026-01-01).
    #[arg(long)]
    pub(crate) fch_resol: Option<String>,

    /// Número de resolución autorizatoria (default: 0).
    #[arg(long, default_value = "0")]
    pub(crate) nro_resol: u32,

    /// Tipo de operación del libro: `compra` o `venta` (default: `venta`).
    #[arg(long, default_value = "venta")]
    pub(crate) operacion: String,

    /// Clasificación del libro: `mensual`, `especial` o `rectifica` (default: `mensual`).
    #[arg(long, default_value = "mensual")]
    pub(crate) tipo_libro: String,

    /// Tipo de envío: `total`, `parcial`, `final` o `ajuste` (default: `total`).
    #[arg(long, default_value = "total")]
    pub(crate) tipo_envio: String,

    /// Archivos DTE XML para agregar automáticamente al libro.
    #[arg(long)]
    pub(crate) dte: Vec<PathBuf>,

    /// Certificado digital para firmar el libro (`.p12`, `.pfx` o `.pem`).
    #[arg(short, long)]
    pub(crate) cert: Option<PathBuf>,

    /// Llave privada en formato PEM si el certificado no la incluye.
    #[arg(long)]
    pub(crate) cert_key: Option<PathBuf>,

    /// Ambiente de destino para la firma del libro (requerido si se firma con `--cert`).
    #[arg(long)]
    pub(crate) ambiente: Option<AmbienteArg>,

    /// Archivo de salida donde guardar el XML generado. Si se omite, se imprime a stdout.
    #[arg(short, long)]
    pub(crate) output: Option<PathBuf>,
}
