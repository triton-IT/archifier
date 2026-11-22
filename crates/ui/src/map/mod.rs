//! Carte de l'entreprise et pages d'objet (cible v0.4).

pub mod commands;
pub mod create;
pub mod page;
pub mod pages;
pub mod palette;
mod viewport;

pub use page::MapPage;
pub use pages::{
    ApplicationPage, BusinessDomainPage, CapabilityPage, ComponentPage, InfrastructurePage,
    MapInterfacePage, ProductPage, TeamPage, ValueStreamPage, ValueStreamStagePage,
};
