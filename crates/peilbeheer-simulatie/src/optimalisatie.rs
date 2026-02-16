use peilbeheer_core::energie::{
    OptimalisatieParams, OptimalisatieResultaat, OptimalisatieUurResultaat,
    SimulatieStapUitgebreid, UurPrijs,
};

use crate::waterbalans::calculate_water_balance;

/// Pompvermogen in kW: P = ρ × g × Q × H / η
/// met ρ = 1000 kg/m³, g = 9.81 m/s²
pub fn calculate_pump_power_kw(debiet_m3s: f64, opvoerhoogte_m: f64, efficiency: f64) -> f64 {
    let eff = if efficiency > 0.0 { efficiency } else { 0.70 };
    1000.0 * 9.81 * debiet_m3s * opvoerhoogte_m / eff / 1000.0 // delen door 1000 voor kW
}

/// Simuleer één uur (60 minuten) met vaste pompfractie.
/// Retourneert de waterstand aan het eind van het uur.
///
/// `effective_regen` is de regenintensiteit geschaald met 1/berging_factor (mm/uur).
/// `oppervlakte` is het volledige peilgebied-oppervlak (m²).
fn simulate_one_hour(
    ws_start: f64,
    pomp_fractie: f64,
    max_debiet: f64,
    effective_regen: f64,
    oppervlakte: f64,
    verdamping: f64,
    infiltratie: f64,
) -> f64 {
    let debiet = pomp_fractie * max_debiet;
    let mut ws = ws_start;

    for _ in 0..60 {
        let balans = calculate_water_balance(effective_regen, oppervlakte, ws, debiet, verdamping, infiltratie);
        ws = balans.nieuwe_waterstand;
    }

    ws
}

/// Simuleer 24 uur met gegeven pompfracties per uur, retourneer gedetailleerde tijdstappen.
fn simulate_24h_detailed(
    params: &OptimalisatieParams,
    pompfracties: &[f64],
    prijzen: &[UurPrijs],
) -> (Vec<SimulatieStapUitgebreid>, f64) {
    let berging = params.berging_factor.max(0.01);

    let mut stappen = Vec::with_capacity(24 * 60);
    let mut ws = params.streefpeil;
    let mut cum_kosten = 0.0;

    for uur in 0..24_usize {
        let fractie = pompfracties[uur];
        let debiet = fractie * params.max_debiet;
        let regen = *params.regen_per_uur.get(uur).unwrap_or(&0.0);
        let effective_regen = regen / berging;
        let prijs = prijzen.get(uur).map(|p| p.prijs_eur_kwh).unwrap_or(0.0);
        let power_kw = calculate_pump_power_kw(debiet, params.opvoerhoogte, params.efficiency);

        for minuut in 0..60 {
            let balans = calculate_water_balance(
                effective_regen,
                params.oppervlakte,
                ws,
                debiet,
                params.verdamping,
                params.infiltratie,
            );

            let kosten_deze_minuut = power_kw * prijs / 60.0; // kWh per minuut
            cum_kosten += kosten_deze_minuut;

            stappen.push(SimulatieStapUitgebreid {
                tijd_minuten: (uur * 60 + minuut) as f64,
                uur: uur as u8,
                waterstand: ws,
                water_afvoer: balans.water_afvoer,
                water_toevoer: balans.water_toevoer,
                is_regen: regen > 0.0,
                is_pomp_aan: debiet > 0.001,
                cumulatieve_kosten: cum_kosten,
                prijs_eur_kwh: prijs,
            });

            ws = balans.nieuwe_waterstand;
        }
    }

    (stappen, cum_kosten)
}

/// Naïef pompschema: pomp 100% als waterstand > streefpeil, 0% als ≤ streefpeil.
fn naive_pump_fractions(
    params: &OptimalisatieParams,
) -> Vec<f64> {
    let berging = params.berging_factor.max(0.01);

    let mut fracties = vec![0.0; 24];
    let mut ws = params.streefpeil;

    for uur in 0..24 {
        let regen = *params.regen_per_uur.get(uur).unwrap_or(&0.0);
        let effective_regen = regen / berging;

        // Bepaal of we moeten pompen: simuleer het uur zonder pomp, kijk of ws stijgt
        let ws_zonder_pomp = simulate_one_hour(
            ws, 0.0, params.max_debiet, effective_regen, params.oppervlakte,
            params.verdamping, params.infiltratie,
        );

        // Naïef: pomp als water boven streefpeil staat
        fracties[uur] = if ws > params.streefpeil + 0.001 {
            1.0
        } else if ws_zonder_pomp > params.streefpeil + 0.001 {
            // Water gaat stijgen, begin te pompen
            1.0
        } else {
            0.0
        };

        // Simuleer het uur met de gekozen fractie
        ws = simulate_one_hour(
            ws, fracties[uur], params.max_debiet, effective_regen, params.oppervlakte,
            params.verdamping, params.infiltratie,
        );
    }

    fracties
}

/// Beschikbare pompfracties voor DP-discretisatie.
const PUMP_FRACTIONS: [f64; 9] = [0.0, 0.05, 0.10, 0.25, 0.40, 0.50, 0.75, 0.90, 1.0];

/// Discretiseer waterstand naar index in de DP state space.
fn ws_to_index(ws: f64, ws_min: f64, stap: f64) -> Option<usize> {
    let idx = ((ws - ws_min) / stap).round() as isize;
    if idx >= 0 { Some(idx as usize) } else { None }
}

/// Index terug naar waterstand.
fn index_to_ws(idx: usize, ws_min: f64, stap: f64) -> f64 {
    ws_min + idx as f64 * stap
}

/// Dynamic Programming optimalisatie van het pompschema.
pub fn optimize_pump_schedule(
    params: &OptimalisatieParams,
) -> Result<OptimalisatieResultaat, String> {
    // Validatie
    if params.oppervlakte <= 0.0 {
        return Err("Oppervlakte moet groter zijn dan 0".into());
    }
    if params.max_debiet <= 0.0 {
        return Err("Max debiet moet groter zijn dan 0".into());
    }
    if params.regen_per_uur.len() != 24 {
        return Err(format!(
            "regen_per_uur moet 24 waarden bevatten, maar bevat {}",
            params.regen_per_uur.len()
        ));
    }

    let berging = params.berging_factor.max(0.01);

    let marge_m = params.marge_cm / 100.0;
    let ws_min = params.streefpeil - marge_m; // band ondergrens
    let ws_max = params.streefpeil + marge_m; // band bovengrens
    let stap = 0.005; // 0.5 cm discretisatie

    // Bereken de maximale verwachte waterstandstijging om de DP-ruimte groot genoeg te maken.
    let total_rain_mm: f64 = params.regen_per_uur.iter().sum();
    let max_rise_m = (total_rain_mm / berging / 1000.0).clamp(0.20, 5.0);

    // DP-toestandsruimte: band + uitloop voor overschrijding
    let ws_dp_min = ws_min - max_rise_m;
    let ws_dp_max = ws_max + max_rise_m;
    let n_niveaus = ((ws_dp_max - ws_dp_min) / stap).round() as usize + 1;

    // Zorg dat er prijzen zijn (24 stuks)
    let prijzen: Vec<UurPrijs> = if params.prijzen.len() == 24 {
        params.prijzen.clone()
    } else {
        // Fallback: uniforme prijs
        (0..24).map(|u| UurPrijs { uur: u, prijs_eur_kwh: 0.10 }).collect()
    };

    // Strafterm: hoge kosten per cm buiten de band, zodat de DP pompen verkiest
    // boven bandoverschrijding.
    let penalty_per_cm = 100.0; // €100 per cm per uur buiten de band

    // DP arrays: kosten[uur][ws_index] = minimale resterende kosten
    // We werken backward: van uur 23 naar uur 0
    let inf = f64::INFINITY;

    // Na uur 23: strafterm voor eindwaterstand buiten band
    let mut next_cost: Vec<f64> = (0..n_niveaus)
        .map(|idx| {
            let ws = index_to_ws(idx, ws_dp_min, stap);
            let overschrijding_cm = if ws < ws_min {
                (ws_min - ws) * 100.0
            } else if ws > ws_max {
                (ws - ws_max) * 100.0
            } else {
                0.0
            };
            overschrijding_cm * penalty_per_cm
        })
        .collect();
    let mut best_fraction: Vec<Vec<f64>> = vec![vec![0.0; n_niveaus]; 24];

    for uur in (0..24).rev() {
        let mut current_cost = vec![inf; n_niveaus];
        let regen = *params.regen_per_uur.get(uur).unwrap_or(&0.0);
        let effective_regen = regen / berging;
        let prijs = prijzen[uur].prijs_eur_kwh;

        for ws_idx in 0..n_niveaus {
            let ws = index_to_ws(ws_idx, ws_dp_min, stap);

            for &fractie in &PUMP_FRACTIONS {
                let debiet = fractie * params.max_debiet;

                // Simuleer dit uur
                let ws_eind = simulate_one_hour(
                    ws, fractie, params.max_debiet, effective_regen, params.oppervlakte,
                    params.verdamping, params.infiltratie,
                );

                // Zoek eind-index in uitgebreide toestandsruimte (geen clamping)
                let eind_idx = match ws_to_index(ws_eind, ws_dp_min, stap) {
                    Some(i) if i < n_niveaus => i,
                    _ => continue,
                };

                // Kosten dit uur: P(kW) × prijs(€/kWh) × 1 uur
                let power_kw = calculate_pump_power_kw(debiet, params.opvoerhoogte, params.efficiency);
                let kosten_uur = power_kw * prijs;

                // Strafterm voor bandoverschrijding
                let overschrijding_cm = if ws_eind < ws_min {
                    (ws_min - ws_eind) * 100.0
                } else if ws_eind > ws_max {
                    (ws_eind - ws_max) * 100.0
                } else {
                    0.0
                };
                let penalty = overschrijding_cm * penalty_per_cm;

                let totaal = kosten_uur + penalty + next_cost[eind_idx];

                if totaal < current_cost[ws_idx] {
                    current_cost[ws_idx] = totaal;
                    best_fraction[uur][ws_idx] = fractie;
                }
            }
        }

        next_cost = current_cost;
    }

    // Forward pass: bepaal optimaal schema vanuit startconditie
    let start_ws = params.streefpeil;
    let start_idx = ws_to_index(start_ws, ws_dp_min, stap)
        .ok_or("Streefpeil valt buiten DP-toestandsruimte")?;
    if start_idx >= n_niveaus {
        return Err("Streefpeil valt buiten DP-toestandsruimte".into());
    }

    let mut opt_fracties = vec![0.0; 24];
    let mut ws = start_ws;
    let mut ws_idx = start_idx;

    for uur in 0..24 {
        let fractie = best_fraction[uur][ws_idx];
        opt_fracties[uur] = fractie;

        let regen = *params.regen_per_uur.get(uur).unwrap_or(&0.0);
        let effective_regen = regen / berging;
        let ws_eind = simulate_one_hour(
            ws, fractie, params.max_debiet, effective_regen, params.oppervlakte,
            params.verdamping, params.infiltratie,
        );

        ws = ws_eind;
        ws_idx = ws_to_index(ws, ws_dp_min, stap)
            .unwrap_or(0)
            .min(n_niveaus - 1);
    }

    // Naïef schema
    let naief_fracties = naive_pump_fractions(params);

    // Simuleer beide schema's gedetailleerd
    let (stappen_opt, kosten_opt) = simulate_24h_detailed(params, &opt_fracties, &prijzen);
    let (stappen_naief, kosten_naief) = simulate_24h_detailed(params, &naief_fracties, &prijzen);

    // Bouw uur-resultaten
    let mut uren = Vec::with_capacity(24);
    let mut max_afwijking_opt: f64 = 0.0;
    let mut max_afwijking_naief: f64 = 0.0;

    for uur in 0..24_usize {
        let regen = *params.regen_per_uur.get(uur).unwrap_or(&0.0);
        let prijs = prijzen[uur].prijs_eur_kwh;

        // Eind waterstand = waterstand aan het einde van het uur
        let minuut_eind = (uur + 1) * 60 - 1;
        let ws_eind_opt = stappen_opt.get(minuut_eind).map(|s| s.waterstand).unwrap_or(params.streefpeil);
        let ws_eind_naief = stappen_naief.get(minuut_eind).map(|s| s.waterstand).unwrap_or(params.streefpeil);

        // Kosten dit uur
        let debiet_opt = opt_fracties[uur] * params.max_debiet;
        let debiet_naief = naief_fracties[uur] * params.max_debiet;
        let power_opt = calculate_pump_power_kw(debiet_opt, params.opvoerhoogte, params.efficiency);
        let power_naief = calculate_pump_power_kw(debiet_naief, params.opvoerhoogte, params.efficiency);
        let kosten_uur_opt = power_opt * prijs;
        let kosten_uur_naief = power_naief * prijs;

        // Max afwijking: bereken over alle minuten van dit uur
        for m in (uur * 60)..((uur + 1) * 60) {
            if let Some(s) = stappen_opt.get(m) {
                let afw = (s.waterstand - params.streefpeil).abs() * 100.0;
                max_afwijking_opt = max_afwijking_opt.max(afw);
            }
            if let Some(s) = stappen_naief.get(m) {
                let afw = (s.waterstand - params.streefpeil).abs() * 100.0;
                max_afwijking_naief = max_afwijking_naief.max(afw);
            }
        }

        uren.push(OptimalisatieUurResultaat {
            uur: uur as u8,
            prijs_eur_kwh: prijs,
            regen_mm_uur: regen,
            pomp_fractie_optimaal: opt_fracties[uur],
            pomp_fractie_naief: naief_fracties[uur],
            waterstand_eind_optimaal: ws_eind_opt,
            waterstand_eind_naief: ws_eind_naief,
            kosten_optimaal: kosten_uur_opt,
            kosten_naief: kosten_uur_naief,
        });
    }

    let besparing = kosten_naief - kosten_opt;
    let besparing_pct = if kosten_naief > 0.001 {
        (besparing / kosten_naief) * 100.0
    } else {
        0.0
    };

    Ok(OptimalisatieResultaat {
        uren,
        totale_kosten_optimaal: kosten_opt,
        totale_kosten_naief: kosten_naief,
        besparing_eur: besparing,
        besparing_pct,
        max_afwijking_optimaal_cm: max_afwijking_opt,
        max_afwijking_naief_cm: max_afwijking_naief,
        tijdstappen_optimaal: stappen_opt,
        tijdstappen_naief: stappen_naief,
        prijzen,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_params(regen: Vec<f64>, prijzen: Vec<f64>) -> OptimalisatieParams {
        let prijzen_vec: Vec<UurPrijs> = prijzen
            .into_iter()
            .enumerate()
            .map(|(i, p)| UurPrijs { uur: i as u8, prijs_eur_kwh: p })
            .collect();
        OptimalisatieParams {
            streefpeil: -0.60,
            max_debiet: 0.5,
            oppervlakte: 100_000.0,
            verdamping: 0.0,
            infiltratie: 0.0,
            opvoerhoogte: 2.0,
            efficiency: 0.70,
            regen_per_uur: regen,
            prijzen: prijzen_vec,
            marge_cm: 20.0,
            berging_factor: 0.10,
        }
    }

    #[test]
    fn test_pump_power() {
        let p = calculate_pump_power_kw(0.5, 2.0, 0.70);
        // P = 1000 * 9.81 * 0.5 * 2.0 / 0.70 / 1000 = 14.01 kW
        assert!((p - 14.014).abs() < 0.1);
    }

    #[test]
    fn test_flat_price_no_rain() {
        // Geen regen, vlakke prijs → optimaal en naïef pompen niks
        let params = make_params(vec![0.0; 24], vec![0.10; 24]);
        let result = optimize_pump_schedule(&params).unwrap();

        assert!((result.totale_kosten_optimaal).abs() < 0.01);
        assert!((result.totale_kosten_naief).abs() < 0.01);
    }

    #[test]
    fn test_flat_price_with_rain() {
        // Regen in uur 6-8, vlakke prijs
        let mut regen = vec![0.0; 24];
        regen[6] = 5.0;
        regen[7] = 10.0;
        regen[8] = 5.0;
        let params = make_params(regen, vec![0.10; 24]);
        let result = optimize_pump_schedule(&params).unwrap();

        // Naïef pompt reactief → kosten > 0
        assert!(
            result.totale_kosten_naief > 0.0,
            "Naief kosten moeten > 0 zijn bij regen, was: {:.4}",
            result.totale_kosten_naief
        );
        // Optimaal kan 0 zijn als de band de regen absorbeert
        assert!(result.totale_kosten_optimaal >= 0.0);
        // Optimaal mag nooit duurder zijn dan naïef
        assert!(result.totale_kosten_optimaal <= result.totale_kosten_naief + 0.01);
    }

    #[test]
    fn test_variable_price_savings() {
        // Regen in uur 10-12, hoge prijs in uur 10-12, lage prijs in uur 6-8
        // Optimizer zou moeten voormalen (pomp draaien op goedkope uren)
        let mut regen = vec![0.0; 24];
        regen[10] = 8.0;
        regen[11] = 8.0;
        regen[12] = 5.0;

        let mut prijzen = vec![0.05; 24]; // goedkoop
        prijzen[10] = 0.30;
        prijzen[11] = 0.30;
        prijzen[12] = 0.25;

        let params = make_params(regen, prijzen);
        let result = optimize_pump_schedule(&params).unwrap();

        // Optimizer zou goedkoper moeten zijn of gelijk (nooit duurder)
        assert!(
            result.totale_kosten_optimaal <= result.totale_kosten_naief + 0.01,
            "Optimaal ({:.4}) mag niet duurder zijn dan naïef ({:.4})",
            result.totale_kosten_optimaal,
            result.totale_kosten_naief
        );
    }

    #[test]
    fn test_constraint_never_violated() {
        // Zware regen, check dat waterstand binnen band blijft
        let mut regen = vec![0.0; 24];
        regen[3] = 5.0;
        regen[4] = 10.0;
        regen[5] = 5.0;

        let params = make_params(regen, vec![0.15; 24]);
        let result = optimize_pump_schedule(&params).unwrap();

        // Max afwijking in cm moet <= marge
        assert!(
            result.max_afwijking_optimaal_cm <= params.marge_cm + 1.0,
            "Optimaal afwijking {:.1} cm overschrijdt marge {:.1} cm",
            result.max_afwijking_optimaal_cm,
            params.marge_cm
        );
    }

    #[test]
    fn test_24_uur_resultaten() {
        let params = make_params(vec![0.0; 24], vec![0.10; 24]);
        let result = optimize_pump_schedule(&params).unwrap();
        assert_eq!(result.uren.len(), 24);
        assert_eq!(result.prijzen.len(), 24);
        assert_eq!(result.tijdstappen_optimaal.len(), 24 * 60);
        assert_eq!(result.tijdstappen_naief.len(), 24 * 60);
    }

    #[test]
    fn test_large_polder_pumps_run() {
        // Groot peilgebied (100 ha) met berging 10%, marge 10 cm
        // De pomp MOET draaien om de waterstand te beheersen
        let mut regen = vec![0.0; 24];
        regen[6] = 10.0;
        regen[7] = 10.0;
        regen[8] = 10.0;
        let mut params = make_params(regen, vec![0.10; 24]);
        params.oppervlakte = 1_000_000.0; // 100 ha
        params.marge_cm = 10.0;
        params.berging_factor = 0.10;

        let result = optimize_pump_schedule(&params).unwrap();

        // Optimaal schema moet pompen (kosten > 0)
        assert!(
            result.totale_kosten_optimaal > 0.0,
            "Optimaal moet pompen bij groot peilgebied met regen, was: €{:.4}",
            result.totale_kosten_optimaal
        );
        // Naïef moet ook pompen
        assert!(
            result.totale_kosten_naief > 0.0,
            "Naief moet pompen, was: €{:.4}",
            result.totale_kosten_naief
        );
        // Optimaal mag nooit duurder zijn dan naïef
        assert!(
            result.totale_kosten_optimaal <= result.totale_kosten_naief + 0.01,
            "Optimaal ({:.2}) mag niet duurder zijn dan naïef ({:.2})",
            result.totale_kosten_optimaal,
            result.totale_kosten_naief
        );
        // Max afwijking moet realistisch zijn (niet honderden cm)
        assert!(
            result.max_afwijking_optimaal_cm < 100.0,
            "Max afwijking optimaal onrealistisch: {:.1} cm",
            result.max_afwijking_optimaal_cm
        );

    }

    #[test]
    fn test_invalid_params() {
        let mut params = make_params(vec![0.0; 24], vec![0.10; 24]);
        params.oppervlakte = 0.0;
        assert!(optimize_pump_schedule(&params).is_err());

        let mut params = make_params(vec![0.0; 24], vec![0.10; 24]);
        params.regen_per_uur = vec![0.0; 12]; // verkeerd aantal
        assert!(optimize_pump_schedule(&params).is_err());
    }
}
