use crate::jql::{ast::JqlAstNode, engine::Engine, error::JqlError, proxy::Proxy};

pub trait Function {
    type ResultType;

    fn execute(&self) -> Result<Self::ResultType, JqlError>;
}

pub struct AccessFunc<'a> {
    value: &'a Proxy<'a>,
    path: &'a str,
}

impl<'a> AccessFunc<'a> {
    pub fn new(value: &'a Proxy, path: &'a str) -> Self {
        Self { value, path }
    }
}

impl<'a> Function for AccessFunc<'a> {
    type ResultType = Proxy<'a>;

    fn execute(&self) -> Result<Proxy<'a>, JqlError> {
        let result = self.value.get(self.path)?;
        Ok(result)
    }
}

pub struct PipeFunc<'a> {
    value: Proxy<'a>,
    source: &'a JqlAstNode<'a>,
    dest: &'a JqlAstNode<'a>,
}

impl<'a> PipeFunc<'a> {
    pub fn new(value: Proxy<'a>, source: &'a JqlAstNode, dest: &'a JqlAstNode) -> Self {
        Self {
            value,
            source,
            dest,
        }
    }
}

impl<'a> Function for PipeFunc<'a> {
    type ResultType = Proxy<'a>;

    fn execute(&self) -> Result<Self::ResultType, JqlError> {
        let o_source = Engine::execute(self.value.clone(), self.source)?;
        let result = Engine::execute(o_source, self.dest)?;
        Ok(result)
    }
}
