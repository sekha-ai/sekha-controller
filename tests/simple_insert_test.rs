use sea_orm::{entity::prelude::*, Database, EntityTrait, Set};

// MINIMAL ENTITY - fully public to satisfy DeriveEntityModel
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "test_items")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub timestamp: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

#[tokio::test]
async fn test_insert_works() {
    let db = Database::connect("sqlite::memory:").await.unwrap();

    db.execute_unprepared("CREATE TABLE test_items (id TEXT PRIMARY KEY, timestamp TEXT NOT NULL)")
        .await
        .unwrap();

    let item = ActiveModel {
        id: Set("test-1".to_string()),
        timestamp: Set("2025-12-27 22:00:00.000".to_string()),
    };

    item.insert(&db).await.unwrap();

    let found = Entity::find_by_id("test-1")
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.timestamp, "2025-12-27 22:00:00.000");
}
