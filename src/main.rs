use rand::prelude::*;
use std::convert::TryInto;
use std::fmt;
use uuid::Uuid;

/// Somebody who tries to hail a `Taxi` will issue a `Request`.
/// A `Request` is therefore represents somebody's desire to be picked up by a `Taxi`.
/// It has a `max_lifetime` which expires the `Request` as if it timed out because it didn't
/// fulfilled quickly enough.
#[derive(Debug, Clone)]
pub struct Request {
    id: Uuid,
    remaining_waiting_time: u64,
    assigned_taxi: Option<Uuid>,
    fulfillment_time: u64,
}

impl Request {
    pub fn new() -> Request {
        Request {
            id: Uuid::new_v4(),
            remaining_waiting_time: 100,
            assigned_taxi: None,
            fulfillment_time: 100,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.remaining_waiting_time > 0 && self.fulfillment_time > 0
    }
}

#[derive(Debug)]
pub struct Taxi {
    id: Uuid,
    is_occupied: bool,
}

impl Taxi {
    pub fn new() -> Taxi {
        Taxi {
            id: Uuid::new_v4(),
            is_occupied: false,
        }
    }
}

#[derive(Debug)]
pub struct World {
    /// How long the `World` updates for in ticks/seconds.
    runtime: u64,

    /// How long the `World` has been running for.
    age: u64,

    /// Change to spawn a request per tick.
    request_spawn_chance: f64,

    /// When the number of `active_requests` reaches this number, no further requests will be
    /// allowed to spawn.
    max_active_requests: u32,

    /// Current `Taxi`s in the `World`.
    taxis: Vec<Taxi>,

    /// Currently active `Request`s in the `World`. These are either being waited for or are
    /// being driven.
    active_requests: Vec<Request>,

    /// Canceled or fulfilled requests. Append only.
    archived_requests: Vec<Request>,

    rng: SmallRng,
}

impl World {
    /// `runtime` is simulation seconds.
    pub fn new(
        runtime: u64,
        request_spawn_chance: f64,
        max_active_requests: u32,
        number_of_taxis: u32,
    ) -> World {
        let taxis = (0..number_of_taxis).map(|_| Taxi::new()).collect();
        let rng = SmallRng::from_rng(thread_rng()).unwrap();

        World {
            runtime,
            age: 0,
            request_spawn_chance,
            max_active_requests,
            taxis,
            active_requests: vec![],
            archived_requests: vec![],
            rng,
        }
    }

    /// Debug print `World` info.
    pub fn info(&self) {
        println!("{}", self);
    }

    /// Spawns requests with a small chance.
    pub fn maybe_spawn_request(&mut self) {
        if self.active_requests.len() < self.max_active_requests.try_into().unwrap()
            && self.rng.gen_bool(self.request_spawn_chance)
        {
            self.active_requests.push(Request::new())
        }
    }

    /// Try to distribute all waiting `Request`s to unoccupied `Taxi`s.
    pub fn distribute_unfulfilled_requests(&mut self) {
        let waiting_requests = self
            .active_requests
            .iter_mut()
            .filter(|r| r.assigned_taxi.is_none());

        for r in waiting_requests {
            let unoccupied_taxi = self.taxis.iter_mut().find(|t| !t.is_occupied);

            if let Some(taxi) = unoccupied_taxi {
                r.assigned_taxi = Some(taxi.id);
                taxi.is_occupied = true;
            } else {
                break;
            }
        }
    }

    /// Update and tick down all `Request`s.
    pub fn update_requests(&mut self) {
        for r in &mut self.active_requests {
            if r.assigned_taxi.is_some() {
                r.fulfillment_time -= 1;
            } else {
                r.remaining_waiting_time -= 1;
            }
        }
    }

    /// Moved `Request`s from `active_requests` to `archived_requests` if they have either:
    /// 1) reached their `fulfillment_time` or
    /// 2) reached their `remaining_waiting_time`.
    pub fn cleanup_requests(&mut self) {
        // First step is to clone all eligible `Request`s from `active_requests` to
        // `archived_requests`.
        for r in &self.active_requests {
            if !r.is_alive() {
                self.archived_requests.push(r.clone());

                // Don't forget to reset the `Taxi` so that it may now take a `Request` again.
                // However, this is only important if this `Request` actually had a `Taxi`
                // assigned. In the case of a canceled `Request`, it didn't have a `Taxi`.
                if let Some(taxi_id) = r.assigned_taxi {
                    let taxi = self
                        .taxis
                        .iter_mut()
                        .find(|t| t.id == taxi_id)
                        .expect("We expected to find a Taxi but didn't find one.");
                    taxi.is_occupied = false;
                }
            }
        }

        // Second step is to bulk delete all th
        self.active_requests.retain(|r| r.is_alive());
    }

    /// Runs until `age` reaches `runtime`.
    pub fn run_till_done(&mut self) {
        while self.age <= self.runtime {
            self.info();
            self.age += 1;

            self.maybe_spawn_request();
            self.distribute_unfulfilled_requests();
            self.update_requests();
            self.cleanup_requests();
        }
    }
}

impl fmt::Display for World {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let num_occupied_taxis = self.taxis.iter().filter(|t| t.is_occupied).count();
        let num_total_taxis = self.taxis.len();
        let num_assigned_requests = self
            .active_requests
            .iter()
            .filter(|r| r.assigned_taxi.is_some())
            .count();
        let num_waiting_requests = self
            .active_requests
            .iter()
            .filter(|r| r.assigned_taxi.is_none())
            .count();
        let num_archived_requests = self.archived_requests.len();
        write!(
            f,
            "Age: {}/{}, Taxis: {} Occ/{} Tot, Requests: {} Asnd/{} Wai/{} Arch",
            self.age,
            self.runtime,
            num_occupied_taxis,
            num_total_taxis,
            num_assigned_requests,
            num_waiting_requests,
            num_archived_requests,
        )
    }
}

fn main() {
    let mut world = World::new(10000, 0.1, 200, 5);
    world.run_till_done();
}
