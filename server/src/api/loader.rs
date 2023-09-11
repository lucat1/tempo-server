use sea_orm::ConnectionTrait;

use super::documents::Included;

#[async_trait::async_trait]
pub trait Loader<E: Send> {
    type Rejection;
    type Entity;
    type Included;

    async fn from_many<C>(db: &C, entities: Vec<E>) -> Result<Self, Self::Rejection>
    where
        Self: Sized,
        C: ConnectionTrait;

    fn entities(self) -> Vec<Self::Entity>;
    fn included<C>(
        &self,
        db: &C,
        included: &[Self::Included],
    ) -> Result<Vec<Included>, Self::Rejection>
    where
        C: ConnectionTrait;
}

#[async_trait::async_trait]
pub trait SingleLoader<E: Send> {
    type Rejection;

    async fn from<C>(db: &C, entity: E) -> Result<Self, Self::Rejection>
    where
        Self: Sized,
        C: ConnectionTrait;
}

#[async_trait::async_trait]
impl<E: Send, L: Loader<E>> SingleLoader<E> for L {
    type Rejection = L::Rejection;

    async fn from<C>(db: &C, entity: E) -> Result<Self, Self::Rejection>
    where
        C: ConnectionTrait,
    {
        Loader::from_many(db, vec![entity]).await
    }
}
