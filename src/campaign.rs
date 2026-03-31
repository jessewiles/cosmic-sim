#![allow(dead_code)]
/// Campaign definitions — structured objectives with narrative bookends.
/// Active campaign is tracked by ID in GameState; looked up from all_campaigns() as needed.

pub struct Campaign {
    pub id:          &'static str,
    pub name:        &'static str,
    pub tagline:     &'static str,
    pub description: &'static str,
    pub objectives:  Vec<CampaignObjective>,
    pub intro:       &'static str,
    pub win_text:    &'static str,
}

pub struct CampaignObjective {
    pub label: &'static str,
    pub kind:  ObjectiveKind,
}

pub enum ObjectiveKind {
    VisitSystem(&'static str),
    #[allow(dead_code)]
    ReachDistance(f64),
}

impl Campaign {
    /// Returns a bool per objective — true if complete given visited system list.
    pub fn objectives_status(&self, visited: &[String], max_dist_ly: f64) -> Vec<bool> {
        self.objectives.iter().map(|obj| match obj.kind {
            ObjectiveKind::VisitSystem(name) => visited.iter().any(|v| v == name),
            ObjectiveKind::ReachDistance(ly) => max_dist_ly >= ly,
        }).collect()
    }

    pub fn is_complete(&self, visited: &[String], max_dist_ly: f64) -> bool {
        self.objectives_status(visited, max_dist_ly).iter().all(|&b| b)
    }

    pub fn completed_count(&self, visited: &[String], max_dist_ly: f64) -> usize {
        self.objectives_status(visited, max_dist_ly).iter().filter(|&&b| b).count()
    }
}

pub fn all_campaigns() -> Vec<Campaign> {
    vec![
        Campaign {
            id:      "core_approach",
            name:    "The Core Approach",
            tagline: "Chart the first corridor toward the galactic center",
            description: "\
The galactic center is 26,000 light-years away. At your ship's current \
velocity, reaching it would take longer than the Sun has left to burn.

That is not the mission.

The mission is the corridor — a sequence of verified waypoints aimed at \
the core, each one a staging post for whoever comes after. Four stars, \
chosen for their alignment with Sagittarius A* and their strategic value \
as refueling anchors. Wolf 359. Ross 128. Arcturus. Regulus.

Someone will finish the route. It may not be you. That is not a reason \
not to start.",

            objectives: vec![
                CampaignObjective {
                    label: "Wolf 359  (7.8 ly)  — first anchor",
                    kind:  ObjectiveKind::VisitSystem("Wolf 359"),
                },
                CampaignObjective {
                    label: "Ross 128  (10.9 ly) — habitable-zone outpost",
                    kind:  ObjectiveKind::VisitSystem("Ross 128"),
                },
                CampaignObjective {
                    label: "Arcturus  (35.6 ly) — deep corridor marker",
                    kind:  ObjectiveKind::VisitSystem("Arcturus"),
                },
                CampaignObjective {
                    label: "Regulus   (79.4 ly) — corridor terminus",
                    kind:  ObjectiveKind::VisitSystem("Regulus"),
                },
            ],

            intro: "\
The fleet's consensus took three weeks of signal exchange to form.

Reza was first — characteristically, he presented it as an inevitability \
rather than a proposal. We are moving anyway, he said. We are already \
aimed somewhere. The question is whether we choose our direction or \
merely drift.

The galactic center. Twenty-six thousand light-years. At 0.1c with \
current drives, unreachable in any practical sense — but charting the \
corridor is not the same as reaching the end of it. Every route that \
has ever mattered began with someone finding the first staging post.

Four waypoints. Wolf 359. Ross 128. Arcturus. Regulus. Each one a \
verified anchor — a place where ships can refuel, take their bearings, \
mark the path.

Someone will finish it. It might not be us. That is not a reason not to start.",

            win_text: "\
Regulus burns blue-white behind you. Seventy-nine light-years from Sol.

You have done what you set out to do. Four waypoints. Four verified \
anchors in the corridor that will — one day — guide ships toward the \
galactic center. Wolf 359, small and ancient. Ross 128, with its quiet \
temperate world at the edge of habitability. Arcturus, a red giant \
crossing the Milky Way disk at high velocity, burning through its last \
few billion years. And Regulus — spinning so fast it flattens at the \
poles, too young to have planets worth landing on, old enough to matter.

The corridor exists now because you made it real.

Yael has already asked about the next leg.",
        },
    ]
}
