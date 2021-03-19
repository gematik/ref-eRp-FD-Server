/*
 * Copyright (c) 2021 gematik GmbH
 * 
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 * 
 *    http://www.apache.org/licenses/LICENSE-2.0
 * 
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
 */

use libxml::{ElementType, NodeRef};
use openssl::{
    pkey::{HasPublic, PKey, PKeyRef, Public},
    x509::X509,
};

use super::{
    C14n, C14nMethod, ChainBuilder, Data, DigestValue, EnvelopedSignature, Error, Hash, HashMethod,
    NodeSet, NodeSetAll, NodeSetLike, NodeSetNone, NodeSetOps, SelectNode, SignatureMethod,
    SignatureValue,
};

pub trait Node {
    fn verify<'a>(&'a self) -> Result<Box<dyn NodeSetLike + 'a>, Error>;
}

macro_rules! read_xml {
    // Move on to the first child element and check if it has the correct name and namespace
    ( begin $iter:expr ) => {{
        $iter.first_child().and_then(NodeRef::next_element)
    }};

    // Check if no siblings are left
    ( end $iter:expr ) => {{
        if let Some(node) = $iter {
            let name = node.name().unwrap_or("<unknown>");

            return Err(Error::InvalidSignatureNode(format!("Expected end but found element '{}'", name)));
        }
    }};

    // Check if the passed node has the correct name and namespace
    ( check $iter:expr, $name:expr, $ns:expr ) => {{
        let name = $iter.name()?;
        if name != $name {
            return Err(Error::InvalidSignatureNode(format!(
                "Expected element '{}' but found '{}'",
                $name, &name
            )));
        }

        let ns = $iter
            .ns()
            .ok_or_else(|| {
                Error::InvalidSignatureNode(format!("Unable to get namespace for '{}'", $name))
            })?
            .href()?;
        if ns != $ns {
            return Err(Error::InvalidSignatureNode(format!(
                "Expected namespace '{}' but found '{}' for element '{}'",
                $ns, &ns, $name
            )));
        }

        $iter.next_sibling().and_then(NodeRef::next_element)
    }};

    // Move on to the next sibling and check if it has the correct name and namespace
    ( next $iter:expr, $name:expr, $ns:expr ) => {{
        let ret = $iter.ok_or_else(|| Error::InvalidSignatureNode(format!("Expected elemtn '{}' but found end", $name)))?;

        $iter = read_xml!(check ret, $name, $ns);

        ret
    }};

    // Move on to the next sibling if it has the given name and check the namespace if so
    ( next_opt $iter:expr, $name:expr, $ns:expr ) => {{
        let node = $iter
            .and_then(|n| match n.name() {
                Ok(name) if name == $name => Some(n),
                _ => None
            });

        match node {
            Some(node) => {
                $iter = read_xml!(check node, $name, $ns);

                Some(node)
            },
            None => None,
        }
    }};

    // Stores all siblings with the given name to a vector
    ( next_vec $iter:expr, $name:expr, $ns:expr ) => {{
        let mut ret = Vec::new();

        loop {
            let node = read_xml!(next_opt $iter, $name, $ns);

            match node {
                Some(node) => ret.push(node),
                None => break,
            }
        }

        ret
    }};
}

impl Node for NodeRef {
    fn verify<'a>(&'a self) -> Result<Box<dyn NodeSetLike + 'a>, Error> {
        let node_signature = self
            .find_element(NODE_SIGNATURE, NAMESPACE_HREF)
            .ok_or(Error::SignatureNodeNotFound)?;

        let mut iter = read_xml!(begin node_signature);
        let node_signed_info = read_xml!(next iter, NODE_SIGNED_INFO, NAMESPACE_HREF);
        let node_signature_value = read_xml!(next iter, NODE_SIGNATURE_VALUE, NAMESPACE_HREF);
        let node_key_info = read_xml!(next iter, NODE_KEY_INFO, NAMESPACE_HREF);
        let _node_objects = read_xml!(next_vec iter, NODE_OBJECT, NAMESPACE_HREF);
        read_xml!(end iter);

        let key = process_key_info(node_key_info)?;
        let signature = process_signature_value(node_signature_value)?;
        let (canonicalization_method, signature_method, node_references) =
            process_signed_info(node_signed_info)?;

        let mut builder = ChainBuilder::default();
        setup_canonization(&mut builder, &canonicalization_method)?;
        setup_signature(&mut builder, &signature_method, &key, signature)?;

        let mut transform = builder.build()?;
        transform.update(Data::Xml(node_signed_info, &NodeSetAll))?;
        transform.finish()?;

        progress_references(node_references)
    }
}

trait NodeEx {
    fn find_element(&self, name: &str, ns_href: &str) -> Option<&NodeRef>;
}

impl NodeEx for NodeRef {
    fn find_element(&self, name: &str, ns_href: &str) -> Option<&NodeRef> {
        self.search(&mut |n| node_matches(n, name, ns_href))
    }
}

pub fn node_matches(node: &NodeRef, name: &str, ns_href: &str) -> bool {
    if node.type_() != ElementType::XML_ELEMENT_NODE {
        return false;
    }

    match node.name() {
        Ok(v) if v == name => (),
        _ => return false,
    }

    let ns = match node.ns() {
        Some(ns) => ns,
        None => return false,
    };

    match ns.href() {
        Ok(v) if v == ns_href => (),
        _ => return false,
    }

    true
}

#[allow(clippy::single_match)]
fn process_key_info(node: &NodeRef) -> Result<PKey<Public>, Error> {
    let mut key = None;
    let mut node = node.first_child().and_then(NodeRef::next_element);
    while let Some(n) = node {
        node = n.next_sibling().and_then(NodeRef::next_element);

        let name = n.name()?;
        let ns = match n.ns() {
            Some(ns) => ns,
            None => continue,
        };
        let ns = ns.href()?;

        match (name, ns) {
            (NODE_X509_DATA, NAMESPACE_HREF) => {
                let mut iter = read_xml!(begin n);
                let node_cert = read_xml!(next iter, NODE_X509_CERTIFICATE, NAMESPACE_HREF);
                read_xml!(end iter);

                let cert = node_cert.content()?.ok_or_else(|| {
                    Error::InvalidSignatureNode(format!(
                        "Node '{}' is missing the certificate content",
                        NODE_X509_CERTIFICATE
                    ))
                })?;
                let cert = format!(
                    "-----BEGIN CERTIFICATE-----\n{}\n-----END CERTIFICATE-----",
                    cert.trim()
                );
                let cert = X509::from_pem(cert.as_bytes())?;

                key = Some(cert.public_key()?);
            }
            _ => (),
        }
    }

    key.ok_or_else(|| Error::InvalidSignatureNode("Unable to find key".into()))
}

fn process_signature_value(node: &NodeRef) -> Result<Data<'static>, Error> {
    let data = node.content()?.ok_or_else(|| {
        Error::InvalidSignatureNode(format!(
            "Node '{}' is missing the signature value",
            NODE_SIGNATURE_VALUE
        ))
    })?;
    let data = data.replace(" ", "");
    let data = data.replace("\n", "");
    let data = Data::Base64(data);

    Ok(data)
}

fn process_signed_info(node: &NodeRef) -> Result<(String, String, Vec<&NodeRef>), Error> {
    let mut iter = read_xml!(begin node);
    let node_canonicalization_method =
        read_xml!(next iter, NODE_CANONICALIZATION_METHOD, NAMESPACE_HREF);
    let node_signature_method = read_xml!(next iter, NODE_SIGNATURE_METHOD, NAMESPACE_HREF);
    let node_references = read_xml!(next_vec iter, NODE_REFERENCE, NAMESPACE_HREF);
    read_xml!(end iter);

    if node_references.is_empty() {
        return Err(Error::InvalidSignatureNode(format!(
            "Node '{}' is empty",
            NODE_REFERENCE
        )));
    }

    let canonicalization_method = node_canonicalization_method
        .prop(PROP_ALGORITHM)?
        .ok_or_else(|| {
            Error::InvalidSignatureNode(format!(
                "Node '{}' is missing the '{}' property",
                NODE_CANONICALIZATION_METHOD, PROP_ALGORITHM
            ))
        })?;
    let signature_method = node_signature_method.prop(PROP_ALGORITHM)?.ok_or_else(|| {
        Error::InvalidSignatureNode(format!(
            "Node '{}' is missing the '{}' property",
            NODE_SIGNATURE_METHOD, PROP_ALGORITHM
        ))
    })?;

    Ok((canonicalization_method, signature_method, node_references))
}

fn progress_references<'a>(nodes: Vec<&'a NodeRef>) -> Result<Box<dyn NodeSetLike + 'a>, Error> {
    let mut ret: Box<dyn NodeSetLike + 'a> = Box::new(NodeSetNone);

    for node in nodes {
        let set = progress_reference(node)?;

        ret = Box::new(ret.union(set))
    }

    Ok(ret)
}

fn progress_reference<'a>(node: &'a NodeRef) -> Result<Box<dyn NodeSetLike + 'a>, Error> {
    let uri = node.prop("URI")?;
    let mut is_enveloped_signature = false;

    let mut iter = read_xml!(begin node);
    let node_transforms = read_xml!(next iter, NODE_TRANSFORMS, NAMESPACE_HREF);
    let node_digest_method = read_xml!(next iter, NODE_DIGEST_METHOD, NAMESPACE_HREF);
    let node_digest_value = read_xml!(next iter, NODE_DIGEST_VALUE, NAMESPACE_HREF);
    read_xml!(end iter);

    let mut iter = read_xml!(begin node_transforms);
    let node_transforms = read_xml!(next_vec iter, NODE_TRANSFORM, NAMESPACE_HREF);
    read_xml!(end iter);

    let mut has_c14n = false;
    let mut builder = ChainBuilder::default();
    builder.append(SelectNode::new(uri.clone()));

    /* Setup Transformations */

    for transform in node_transforms {
        let alg = transform.prop(PROP_ALGORITHM)?.ok_or_else(|| {
            Error::InvalidSignatureNode(format!(
                "Node '{}' is missing the '{}' property",
                NODE_TRANSFORM, PROP_ALGORITHM
            ))
        })?;

        setup_transform(
            &mut builder,
            transform,
            &alg,
            Some(&mut has_c14n),
            Some(&mut is_enveloped_signature),
        )?;
    }

    if !has_c14n {
        builder.append(C14n::new(C14nMethod::C14n_1_0));
    }

    /* Setup Digest Method */

    let digest_method = node_digest_method.prop(PROP_ALGORITHM)?.ok_or_else(|| {
        Error::InvalidSignatureNode(format!(
            "Node '{}' is missing the '{}' property",
            NODE_DIGEST_METHOD, PROP_ALGORITHM
        ))
    })?;

    setup_digest(&mut builder, &digest_method)?;

    /* Setup Digest Value */

    let digest_value = node_digest_value.content()?.ok_or_else(|| {
        Error::InvalidSignatureNode(format!(
            "Node '{}' is missing the certificate content",
            NODE_DIGEST_VALUE
        ))
    })?;
    let digest_value = Data::Base64(digest_value);

    builder.append(DigestValue::new(digest_value));

    /* Execute Transformation */

    let node_root = node.doc()?.root()?;
    let node_ref = node_root
        .xpath(uri.as_deref())?
        .ok_or(Error::UnableToGetNodeForXPath(uri))?;
    let node_signature = node
        .search_parent(|n| node_matches(n, NODE_SIGNATURE, NAMESPACE_HREF))
        .ok_or(Error::SignatureNodeNotFound)?;

    let mut transform = builder.build().unwrap();
    transform.update(Data::Xml(node_root, &NodeSetAll))?;
    transform.finish()?;

    let mut node_set: Box<dyn NodeSetLike> = Box::new(NodeSet::from_node(node_ref)?);
    if node_signature.has_parent(node_ref) {
        node_set = Box::new(node_set.complement(NodeSet::from_node(node_signature)?));
    }

    Ok(node_set)
}

fn setup_canonization(
    builder: &mut ChainBuilder,
    canonicalization_method: &str,
) -> Result<(), Error> {
    match canonicalization_method {
        TRANSFORM_C14N_1_0 => {
            builder.append(C14n::new(C14nMethod::C14n_1_0));
        }
        TRANSFORM_C14N_1_0_EXCLUSIVE => {
            builder.append(C14n::new(C14nMethod::C14n_Exclusive_1_0));
        }
        x => return Err(Error::UnknownCanonizationMethod(x.into())),
    }

    Ok(())
}

fn setup_signature<'a, T>(
    builder: &mut ChainBuilder<'a>,
    signature_method: &str,
    key: &'a PKeyRef<T>,
    signature: Data<'static>,
) -> Result<(), Error>
where
    T: HasPublic,
{
    match signature_method {
        SIGNATURE_RSA_SHA1 => {
            builder.append(SignatureValue::new(
                key,
                SignatureMethod::RsaSha1,
                signature,
            ));
        }
        SIGNATURE_RSA_MGF_SHA256 => {
            builder.append(SignatureValue::new(
                key,
                SignatureMethod::RsaMgfSha256,
                signature,
            ));
        }
        x => return Err(Error::UnknownSignatureMethod(x.into())),
    }

    Ok(())
}

fn setup_transform<'a>(
    builder: &mut ChainBuilder<'a>,
    node: &'a NodeRef,
    transform: &str,
    has_c14n: Option<&mut bool>,
    is_enveloped_signature: Option<&mut bool>,
) -> Result<(), Error> {
    match transform {
        TRANSFORM_ENVELOPED_SIGNATURE => {
            builder.append(EnvelopedSignature::new(node));

            if let Some(is_enveloped_signature) = is_enveloped_signature {
                *is_enveloped_signature = true;
            }
        }
        TRANSFORM_C14N_1_0 => {
            builder.append(C14n::new(C14nMethod::C14n_1_0));

            if let Some(has_c14n) = has_c14n {
                *has_c14n = true;
            }
        }
        TRANSFORM_C14N_1_0_EXCLUSIVE => {
            builder.append(C14n::new(C14nMethod::C14n_Exclusive_1_0));

            if let Some(has_c14n) = has_c14n {
                *has_c14n = true;
            }
        }
        x => return Err(Error::UnknownTransformation(x.into())),
    }

    Ok(())
}

fn setup_digest<'a>(builder: &mut ChainBuilder<'a>, digest_method: &str) -> Result<(), Error> {
    match digest_method {
        DIGEST_SHA1 => {
            builder.append(Hash::new(HashMethod::Sha1));
        }
        DIGEST_SHA256 => {
            builder.append(Hash::new(HashMethod::Sha256));
        }
        x => return Err(Error::UnknownDigestMethod(x.into())),
    }

    Ok(())
}

pub const NODE_SIGNATURE: &str = "Signature";
pub const NODE_SIGNED_INFO: &str = "SignedInfo";
pub const NODE_SIGNATURE_VALUE: &str = "SignatureValue";
pub const NODE_KEY_INFO: &str = "KeyInfo";
pub const NODE_OBJECT: &str = "Object";
pub const NODE_CANONICALIZATION_METHOD: &str = "CanonicalizationMethod";
pub const NODE_SIGNATURE_METHOD: &str = "SignatureMethod";
pub const NODE_REFERENCE: &str = "Reference";
pub const NODE_X509_DATA: &str = "X509Data";
pub const NODE_X509_CERTIFICATE: &str = "X509Certificate";
pub const NODE_TRANSFORMS: &str = "Transforms";
pub const NODE_TRANSFORM: &str = "Transform";
pub const NODE_DIGEST_METHOD: &str = "DigestMethod";
pub const NODE_DIGEST_VALUE: &str = "DigestValue";

pub const PROP_ALGORITHM: &str = "Algorithm";

pub const NAMESPACE_HREF: &str = "http://www.w3.org/2000/09/xmldsig#";

pub const TRANSFORM_C14N_1_0: &str = "http://www.w3.org/TR/2001/REC-xml-c14n-20010315";
pub const TRANSFORM_C14N_1_0_EXCLUSIVE: &str = "http://www.w3.org/2001/10/xml-exc-c14n#";
pub const TRANSFORM_ENVELOPED_SIGNATURE: &str =
    "http://www.w3.org/2000/09/xmldsig#enveloped-signature";

pub const DIGEST_SHA1: &str = "http://www.w3.org/2000/09/xmldsig#sha1";
pub const DIGEST_SHA256: &str = "http://www.w3.org/2001/04/xmlenc#sha256";

pub const SIGNATURE_RSA_SHA1: &str = "http://www.w3.org/2000/09/xmldsig#rsa-sha1";
pub const SIGNATURE_RSA_MGF_SHA256: &str = "http://www.w3.org/2007/05/xmldsig-more#sha256-rsa-MGF1";
