use std::fmt::Write;

use crate::{
    OMSerializable,
    ser::{OMAttrSerializable, OMForeignSerializable},
};

#[derive(Debug, thiserror::Error)]
pub enum XmlWriteError {
    #[error("error converting OpenMath: {0}")]
    Custom(String),
    #[error("fmt error")]
    Fmt(#[from] std::fmt::Error),
}
impl super::Error for XmlWriteError {
    fn custom(err: impl std::fmt::Display) -> Self {
        Self::Custom(err.to_string())
    }
}

pub struct XmlDisplay<'s, O: super::OMSerializable + ?Sized> {
    pub pretty: bool,
    pub o: &'s O,
}
impl<O: super::OMSerializable + ?Sized> std::fmt::Display for XmlDisplay<'_, O> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let displayer = XmlDisplayer {
            indent: if self.pretty { Some((false, 0)) } else { None },
            w: f,
            next_ns: self.o.cd_base(),
            current_ns: crate::OPENMATH_BASE_URI.as_str(),
        };
        self.o.as_openmath(displayer).map_err(|_| std::fmt::Error)
    }
}

pub struct XmlObjDisplay<'s, O: super::OMSerializable + ?Sized> {
    pub pretty: bool,
    pub insert_namespace: bool,
    pub o: &'s O,
}
impl<O: super::OMSerializable + ?Sized> std::fmt::Display for XmlObjDisplay<'_, O> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<OMOBJ version=\"2.0\"")?;
        if self.insert_namespace {
            f.write_str(" xmlns=\"")?;
            f.write_str(crate::XML_NAMESPACE)?;
            f.write_char('\"')?;
        }
        let ns = if let Some(ns) = self.o.cd_base() {
            f.write_str("cdbase=\"")?;
            write!(DisplayEscaper(f), "{ns}")?;
            f.write_str("\"")?;
            ns
        } else {
            crate::OPENMATH_BASE_URI.as_str()
        };
        f.write_char('>')?;

        self.o
            .as_openmath(XmlDisplayer {
                indent: if self.pretty { Some((true, 1)) } else { None },
                w: f,
                next_ns: None,
                current_ns: ns,
            })
            .map_err(|_| std::fmt::Error)?;

        if self.pretty {
            f.write_str("\n</OMOBJ>")?;
        } else {
            f.write_str("</OMOBJ>")?;
        }
        Ok(())
    }
}

struct XmlDisplayer<'s, 'f: 's> {
    indent: Option<(bool, usize)>,
    w: &'s mut std::fmt::Formatter<'f>,
    next_ns: Option<&'s str>,
    current_ns: &'s str,
}
impl<'f> XmlDisplayer<'_, 'f> {
    fn indent(&mut self) -> std::fmt::Result {
        let Some((had_content, indent)) = self.indent else {
            return Ok(());
        };
        if had_content {
            self.w.write_str("\n")?;
        }
        self.indent = Some((true, indent));
        for _ in 0..indent {
            self.w.write_str("  ")?;
        }
        Ok(())
    }

    fn indented(
        &mut self,
        f: impl FnOnce(&mut Self) -> Result<(), XmlWriteError>,
    ) -> Result<(), XmlWriteError> {
        if let Some((_, v)) = self.indent.as_mut() {
            *v += 1;
        }
        let r = f(self);
        if let Some((_, v)) = self.indent.as_mut() {
            *v -= 1;
        }
        r
    }

    #[inline]
    const fn clone(&mut self) -> XmlDisplayer<'_, 'f> {
        XmlDisplayer {
            indent: self.indent,
            w: self.w,
            next_ns: self.next_ns,
            current_ns: self.current_ns,
        }
    }

    fn omforeign<OM: OMSerializable, F: std::fmt::Display>(
        &mut self,
        a: &OMForeignSerializable<'_, OM, F>,
    ) -> Result<(), XmlWriteError> {
        match a {
            OMForeignSerializable::OM(o) => o.as_openmath(self.clone())?,
            OMForeignSerializable::Foreign { encoding, value } => {
                let ind = self.indent.is_some();
                if ind {
                    self.indent()?;
                }
                if let Some(enc) = encoding {
                    self.w.write_str("<OMFOREIGN encoding=\"")?;
                    write!(DisplayEscaper(self.w), "{enc}")?;
                    self.w.write_str("\">")?;
                } else {
                    self.w.write_str("<OMFOREIGN>")?;
                }
                if ind {
                    self.indent()?;
                    write!(DisplayEscaper(self.w), "  {value}")?;
                    self.indent()?;
                } else {
                    write!(DisplayEscaper(self.w), "{value}")?;
                }
                self.w.write_str("</OMFOREIGN>")?;
            }
        }
        Ok(())
    }
}

impl<'s, 'f> super::OMSerializer<'s> for XmlDisplayer<'s, 'f> {
    type Ok = ();
    type Err = XmlWriteError;
    type SubSerializer<'ns>
        = XmlDisplayer<'ns, 'f>
    where
        's: 'ns;
    #[inline]
    fn current_cd_base(&self) -> &str {
        self.next_ns.unwrap_or(self.current_ns)
    }
    fn with_cd_base<'ns>(self, cd_base: &'ns str) -> Result<Self::SubSerializer<'ns>, Self::Err>
    where
        's: 'ns,
    {
        if self.current_ns == cd_base {
            Ok(self)
        } else {
            Ok(XmlDisplayer {
                indent: self.indent,
                w: self.w,
                next_ns: Some(cd_base),
                current_ns: self.current_ns,
            })
        }
    }
    fn omi(mut self, value: &crate::Int) -> Result<Self::Ok, Self::Err> {
        self.indent()?;
        write!(self.w, "<OMI>{value}</OMI>")?;
        Ok(())
    }
    fn omf(mut self, value: f64) -> Result<Self::Ok, Self::Err> {
        self.indent()?;
        write!(self.w, "<OMF dec=\"{value}\"/>")?;
        Ok(())
    }
    fn omb<I: IntoIterator<Item = u8>>(mut self, bytes: I) -> Result<Self::Ok, Self::Err>
    where
        I::IntoIter: ExactSizeIterator,
    {
        use crate::base64::Base64Encodable;
        self.indent()?;
        self.w.write_str("<OMB>")?;
        for [a, b, c, d] in bytes.into_iter().base64() {
            self.w.write_char(a.get() as _)?;
            self.w.write_char(b.get() as _)?;
            self.w.write_char(c.get() as _)?;
            self.w.write_char(d.get() as _)?;
        }
        self.w.write_str("</OMB>")?;
        Ok(())
    }
    fn omstr(mut self, string: &impl std::fmt::Display) -> Result<Self::Ok, Self::Err> {
        self.indent()?;
        self.w.write_str("<OMSTR>")?;
        write!(DisplayEscaper(self.w), "{string}")?;
        self.w.write_str("</OMSTR>")?;
        Ok(())
    }
    fn omv(mut self, name: &impl std::fmt::Display) -> Result<Self::Ok, Self::Err> {
        self.indent()?;
        self.w.write_str("<OMV name=\"")?;
        write!(DisplayEscaper(self.w), "{name}")?;
        self.w.write_str("\"/>")?;
        Ok(())
    }
    fn oms(
        mut self,
        cd_name: &impl std::fmt::Display,
        name: &impl std::fmt::Display,
    ) -> Result<Self::Ok, Self::Err> {
        self.indent()?;
        self.w.write_str("<OMS ")?;
        if let Some(cdbase) = self.next_ns {
            self.w.write_str("cdbase=\"")?;
            write!(DisplayEscaper(self.w), "{cdbase}")?;
            self.w.write_str("\" ")?;
        }
        self.w.write_str("cd=\"")?;
        write!(DisplayEscaper(self.w), "{cd_name}")?;
        self.w.write_str("\" name=\"")?;
        write!(DisplayEscaper(self.w), "{name}")?;
        self.w.write_str("\"/>")?;
        Ok(())
    }
    fn ome<
        'a,
        T: super::OMSerializable + 'a,
        D: std::fmt::Display + 'a,
        I: IntoIterator<Item = super::OMForeignSerializable<'a, T, D>>,
    >(
        mut self,
        cd_name: &impl std::fmt::Display,
        name: &impl std::fmt::Display,
        args: I,
    ) -> Result<Self::Ok, Self::Err>
    where
        I::IntoIter: ExactSizeIterator,
    {
        self.indent()?;
        if let Some(ns) = self.next_ns.take() {
            self.w.write_str("<OME cdbase=\"")?;
            write!(DisplayEscaper(self.w), "{ns}")?;
            self.w.write_str("\">")?;
            self.current_ns = ns;
        } else {
            self.w.write_str("<OME>")?;
        }
        self.indented(|nslf| {
            nslf.clone().oms(cd_name, name)?;
            for a in args {
                nslf.omforeign(&a)?;
            }
            Ok(())
        })?;
        self.indent()?;
        self.w.write_str("</OME>")?;
        Ok(())
    }
    fn oma<'a, T: super::OMSerializable + 'a, I: IntoIterator<Item = &'a T>>(
        mut self,
        head: &'a impl super::OMSerializable,
        args: I,
    ) -> Result<Self::Ok, Self::Err>
    where
        I::IntoIter: ExactSizeIterator,
    {
        self.indent()?;
        if let Some(ns) = self.next_ns.take() {
            self.w.write_str("<OMA cdbase=\"")?;
            write!(DisplayEscaper(self.w), "{ns}")?;
            self.w.write_str("\">")?;
            self.current_ns = ns;
        } else {
            self.w.write_str("<OMA>")?;
        }
        self.indented(|nslf| {
            head.as_openmath(nslf.clone())?;
            for a in args {
                a.as_openmath(nslf.clone())?;
            }
            Ok(())
        })?;
        self.indent()?;
        self.w.write_str("</OMA>")?;
        Ok(())
    }

    fn omattr<
        'a,
        T: super::OMSerializable + 'a,
        D: std::fmt::Display + 'a,
        I: IntoIterator<Item = &'a super::OMAttrSerializable<'a, T, D>>,
    >(
        mut self,
        attrs: I,
        atp: &'a impl super::OMSerializable,
    ) -> Result<Self::Ok, Self::Err>
    where
        I::IntoIter: ExactSizeIterator,
    {
        let attrs = attrs.into_iter();
        if attrs.len() == 0 {
            return atp.as_openmath(self.clone());
        }

        self.indent()?;
        if let Some(ns) = self.next_ns.take() {
            self.w.write_str("<OMATTR cdbase=\"")?;
            write!(DisplayEscaper(self.w), "{ns}")?;
            self.w.write_str("\">")?;
            self.current_ns = ns;
        } else {
            self.w.write_str("<OMATTR>")?;
        }

        self.indented(move |nslf| {
            nslf.indent()?;
            nslf.w.write_str("<OMATP>")?;
            nslf.indented(move |nslf| {
                for OMAttrSerializable {
                    cdbase,
                    cd,
                    name,
                    value,
                } in attrs
                {
                    let mut s = nslf.clone();
                    if let Some(cdbase) = cdbase {
                        s.next_ns = Some(cdbase);
                    }
                    s.oms(cd, name)?;
                    nslf.omforeign(value)?;
                }
                Ok(())
            })?;
            nslf.indent()?;
            nslf.w.write_str("</OMATP>")?;
            Ok(())
        })?;

        atp.as_openmath(self.clone())?;
        self.indent()?;
        self.w.write_str("</OMATTR>")?;
        Ok(())
    }

    fn ombind<
        'a,
        St: std::fmt::Display + 'a,
        O: super::OMSerializable + 'a,
        I: IntoIterator<Item = (&'a St, &'a [&'a super::OMAttrSerializable<'a, O, St>])>,
    >(
        mut self,
        head: &'a impl super::OMSerializable,
        vars: I,
        body: &'a impl super::OMSerializable,
    ) -> Result<Self::Ok, Self::Err>
    where
        I::IntoIter: ExactSizeIterator,
    {
        self.indent()?;
        if let Some(ns) = self.next_ns.take() {
            self.w.write_str("<OMBIND cdbase=\"")?;
            write!(DisplayEscaper(self.w), "{ns}")?;
            self.w.write_str("\">")?;
            self.current_ns = ns;
        } else {
            self.w.write_str("<OMBIND>")?;
        }

        self.indented(|nslf| {
            head.as_openmath(nslf.clone())?;
            nslf.indent()?;
            nslf.w.write_str("<OMBVAR")?;
            let mut was_empty = true;

            nslf.indented(|nslf| {
                for (v, attrs) in vars {
                    if was_empty {
                        nslf.w.write_char('>')?;
                    }
                    was_empty = false;
                    if attrs.is_empty() {
                        nslf.clone().omv(v)?;
                    } else {
                        nslf.clone().omattr(attrs.iter().copied(), &super::Omv(v))?;
                    }
                }
                Ok(())
            })?;
            if was_empty {
                nslf.w.write_str("/>")?;
            } else {
                nslf.indent()?;
                nslf.w.write_str("</OMBVAR>")?;
            }
            body.as_openmath(nslf.clone())
        })?;

        self.indent()?;
        self.w.write_str("</OMBIND>")?;
        Ok(())
    }
}

struct DisplayEscaper<'a, 'f>(&'a mut std::fmt::Formatter<'f>);
impl std::fmt::Write for DisplayEscaper<'_, '_> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        let mut is_first = true;
        for seq in s.split('&') {
            if !is_first {
                self.0.write_str("&amp;")?;
            }
            is_first = false;
            let mut is_first = true;
            for seq in seq.split('<') {
                if !is_first {
                    self.0.write_str("&lt;")?;
                }
                is_first = false;
                self.0.write_str(seq)?;
            }
        }
        Ok(())
    }
    fn write_char(&mut self, c: char) -> std::fmt::Result {
        match c {
            '&' => self.0.write_str("&amp;"),
            '<' => self.0.write_str("&lt;"),
            _ => self.0.write_char(c),
        }
    }
}
