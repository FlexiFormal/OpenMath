use quick_xml::events::{BytesStart, Event};

#[derive(Debug, Clone, thiserror::Error)]
pub enum ReadError {
    #[error("{error} (at offset {position})")]
    Xml {
        error: quick_xml::errors::Error,
        position: u64,
    },
    #[error("invalid empty element at {0}")]
    Empty(u64),
    #[error("unknown OpenMath element at {0}")]
    UnknownTag(u64),
    #[error("missing OpenMath object")]
    NoObject,
}

pub(super) fn read<O: for<'a> super::OMDeserializable<'a>>(
    reader: impl std::io::BufRead,
) -> Result<O, ReadError> {
    let mut reader = quick_xml::Reader::from_reader(reader);
    let mut buf = Vec::with_capacity(1024);
    loop {
        let now = reader.buffer_position();
        match reader
            .read_event_into(&mut buf)
            .map_err(|e| ReadError::Xml {
                error: e,
                position: reader.error_position(),
            })? {
            Event::Start(s) => {
                /*let id = s.attributes().find_map(|a| {
                    a.ok().and_then(|a| {
                        if a.key.local_name().as_ref() == b"id" {
                            std::str::from_utf8(a.value.as_ref()).ok()
                            //.map(ToString::to_string)
                        } else {
                            None
                        }
                    })
                });*/

                match s.local_name().as_ref() {
                    b"OMI" => todo!(),
                    b"OMF" => todo!(),
                    b"OMSTR" => todo!(),
                    b"OMB" => todo!(),
                    b"OMV" => todo!(),
                    b"OMS" => todo!(),
                    b"OME" => todo!(),
                    b"OMA" => todo!(),
                    b"OMBIND" => todo!(),
                    _ => return Err(ReadError::UnknownTag(now)),
                }
            }
            Event::Empty(_) => {
                return Err(ReadError::Empty(now));
            }
            Event::Eof => return Err(ReadError::NoObject),
            _ => (),
        }
        buf.clear();
    }
}

#[test]
fn xml_wut() {
    let _ = tracing_subscriber::fmt().try_init();
    let s = r#"<xml xmlns="http://www.openmath.org/OpenMath" xmlns:om="http://www.openmath.org/OpenMath"><test>foo</test><om:test om:foo="bar"/></xml>"#;
    let mut reader = quick_xml::Reader::from_str(s);
    loop {
        match reader.read_event().expect("works") {
            Event::Eof => break,
            Event::Start(e) => {
                tracing::info!(
                    "{:?}={:?}={:?}",
                    e.name(),
                    e.local_name(),
                    e.name().prefix()
                );
                for a in e.attributes() {
                    let a = a.expect("wut");
                    tracing::info!("{:?}={:?}", a.key, a.key.as_namespace_binding());
                }
            }
            Event::Empty(e) => {
                tracing::info!(
                    "{:?}={:?}={:?}",
                    e.name(),
                    e.local_name(),
                    e.name().prefix()
                );
            }
            _ => (),
        }
    }
}
