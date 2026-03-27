//! Tests for dependency resolution (topological sort) extracted from Gearbox::crank().

#[cfg(feature = "testing")]
mod tests {
    use gearbox_rs_core::{resolve_init_order, BoxFuture, CogFactory, Error, Hub};
    use std::any::{Any, TypeId};
    use std::collections::HashMap;
    use std::sync::Arc;

    // Marker types for fake factories
    struct ServiceA;
    struct ServiceB;
    struct ServiceC;

    struct FakeFactory {
        type_id: TypeId,
        name: &'static str,
        deps: Vec<TypeId>,
    }

    impl CogFactory for FakeFactory {
        fn type_id(&self) -> TypeId {
            self.type_id
        }
        fn type_name(&self) -> &'static str {
            self.name
        }
        fn deps(&self) -> Vec<TypeId> {
            self.deps.clone()
        }
        fn build(
            &self,
            _hub: Arc<Hub>,
        ) -> BoxFuture<'static, Result<Arc<dyn Any + Send + Sync>, Error>> {
            Box::pin(async { Ok(Arc::new(()) as Arc<dyn Any + Send + Sync>) })
        }
    }

    #[test]
    fn test_empty_factories() {
        let factories: HashMap<TypeId, &dyn CogFactory> = HashMap::new();
        let order = resolve_init_order(&factories).unwrap();
        assert!(order.is_empty());
    }

    #[test]
    fn test_single_factory_no_deps() {
        let factory = FakeFactory {
            type_id: TypeId::of::<ServiceA>(),
            name: "ServiceA",
            deps: vec![],
        };
        let mut factories: HashMap<TypeId, &dyn CogFactory> = HashMap::new();
        factories.insert(factory.type_id, &factory);

        let order = resolve_init_order(&factories).unwrap();
        assert_eq!(order.len(), 1);
        assert_eq!(order[0], TypeId::of::<ServiceA>());
    }

    #[test]
    fn test_linear_dependency_chain() {
        // C depends on B, B depends on A
        let fa = FakeFactory {
            type_id: TypeId::of::<ServiceA>(),
            name: "ServiceA",
            deps: vec![],
        };
        let fb = FakeFactory {
            type_id: TypeId::of::<ServiceB>(),
            name: "ServiceB",
            deps: vec![TypeId::of::<ServiceA>()],
        };
        let fc = FakeFactory {
            type_id: TypeId::of::<ServiceC>(),
            name: "ServiceC",
            deps: vec![TypeId::of::<ServiceB>()],
        };

        let mut factories: HashMap<TypeId, &dyn CogFactory> = HashMap::new();
        factories.insert(fa.type_id, &fa);
        factories.insert(fb.type_id, &fb);
        factories.insert(fc.type_id, &fc);

        let order = resolve_init_order(&factories).unwrap();
        assert_eq!(order.len(), 3);

        // A must come before B, B must come before C
        let pos_a = order.iter().position(|&id| id == TypeId::of::<ServiceA>()).unwrap();
        let pos_b = order.iter().position(|&id| id == TypeId::of::<ServiceB>()).unwrap();
        let pos_c = order.iter().position(|&id| id == TypeId::of::<ServiceC>()).unwrap();
        assert!(pos_a < pos_b);
        assert!(pos_b < pos_c);
    }

    #[test]
    fn test_cyclic_dependency_detected() {
        // A depends on B, B depends on A
        let fa = FakeFactory {
            type_id: TypeId::of::<ServiceA>(),
            name: "ServiceA",
            deps: vec![TypeId::of::<ServiceB>()],
        };
        let fb = FakeFactory {
            type_id: TypeId::of::<ServiceB>(),
            name: "ServiceB",
            deps: vec![TypeId::of::<ServiceA>()],
        };

        let mut factories: HashMap<TypeId, &dyn CogFactory> = HashMap::new();
        factories.insert(fa.type_id, &fa);
        factories.insert(fb.type_id, &fb);

        let result = resolve_init_order(&factories);
        assert!(result.is_err());
        match result {
            Err(Error::CyclicDependency(msg)) => {
                assert!(msg.contains("ServiceA") || msg.contains("ServiceB"));
            }
            other => panic!("Expected CyclicDependency, got {:?}", other),
        }
    }

    #[test]
    fn test_missing_dependency_detected() {
        // A depends on B, but B is not registered
        let fa = FakeFactory {
            type_id: TypeId::of::<ServiceA>(),
            name: "ServiceA",
            deps: vec![TypeId::of::<ServiceB>()],
        };

        let mut factories: HashMap<TypeId, &dyn CogFactory> = HashMap::new();
        factories.insert(fa.type_id, &fa);

        let result = resolve_init_order(&factories);
        assert!(result.is_err());
        match result {
            Err(Error::MissingDependency(name, _)) => {
                assert_eq!(name, "ServiceA");
            }
            other => panic!("Expected MissingDependency, got {:?}", other),
        }
    }
}
