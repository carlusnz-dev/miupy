//! Fila de reprodução: lista de ids e a posição atual. Lógica pura, sem áudio nem banco.

// @req RF-03

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlayQueue {
    ids: Vec<String>,
    original_ids: Vec<String>,
    index: Option<usize>,
}

impl PlayQueue {
    pub fn ids(&self) -> &[String] {
        &self.ids
    }

    pub fn index(&self) -> Option<usize> {
        self.index
    }

    /// Índice no formato do contrato (-1 se vazia ou sem posição).
    pub fn index_i64(&self) -> i64 {
        self.index.map_or(-1, |i| i as i64)
    }

    #[cfg(test)]
    pub fn current(&self) -> Option<&str> {
        self.index.and_then(|i| self.ids.get(i)).map(String::as_str)
    }

    pub fn id_at(&self, i: usize) -> Option<&str> {
        self.ids.get(i).map(String::as_str)
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// Troca a fila inteira e aponta para `current`. Se `current` não estiver na lista, entra
    /// no começo (quem pede para tocar uma faixa espera que ela toque).
    pub fn replace(&mut self, mut ids: Vec<String>, current: &str) {
        let pos = match ids.iter().position(|i| i == current) {
            Some(p) => p,
            None => {
                ids.insert(0, current.to_owned());
                0
            }
        };
        self.original_ids = ids.clone();
        self.ids = ids;
        self.index = Some(pos);
    }

    // @req RF-24
    /// Troca a ordem sem perder a posição da ocorrência atual, mesmo com IDs repetidos.
    pub fn set_ids(&mut self, ids: Vec<String>, index: usize) {
        self.ids = ids;
        self.index = (index < self.ids.len()).then_some(index);
    }

    // @req RF-24
    /// Ordena as ocorrências pendentes, descontando as já tocadas da fila original.
    pub fn original_order_of(&self, played: &[String], remaining: &[String]) -> Vec<String> {
        let mut played_counts = std::collections::HashMap::<&str, usize>::new();
        for id in played {
            *played_counts.entry(id).or_default() += 1;
        }
        let mut counts = std::collections::HashMap::<&str, usize>::new();
        for id in remaining {
            *counts.entry(id).or_default() += 1;
        }
        let mut ordered = self
            .original_ids
            .iter()
            .filter(|id| {
                if let Some(played_count) = played_counts.get_mut(id.as_str())
                    && *played_count > 0
                {
                    *played_count -= 1;
                    return false;
                }
                let Some(count) = counts.get_mut(id.as_str()) else {
                    return false;
                };
                if *count == 0 {
                    return false;
                }
                *count -= 1;
                true
            })
            .cloned()
            .collect::<Vec<_>>();
        // Preserva todas as ocorrências se a fila original estiver desatualizada.
        for id in remaining {
            let Some(count) = counts.get_mut(id.as_str()) else {
                continue;
            };
            if *count > 0 {
                ordered.push(id.clone());
                *count -= 1;
            }
        }
        ordered
    }

    /// Aponta para `track_id` se ele já estiver na fila (a primeira ocorrência).
    pub fn select(&mut self, track_id: &str) -> bool {
        match self.ids.iter().position(|i| i == track_id) {
            Some(p) => {
                self.index = Some(p);
                true
            }
            None => false,
        }
    }

    pub fn set_index(&mut self, i: usize) {
        if i < self.ids.len() {
            self.index = Some(i);
        }
    }

    pub fn next_after(&self, i: usize) -> Option<usize> {
        (i + 1 < self.ids.len()).then_some(i + 1)
    }

    pub fn prev_before(&self, i: usize) -> Option<usize> {
        i.checked_sub(1)
    }

    // @req RF-03 RF-24
    /// Remove todas as ocorrências de `track_id`. Devolve `true` se a faixa atual foi removida;
    /// nesse caso o índice passa para a faixa que ocupava o lugar dela (ou `None` no fim).
    pub fn remove(&mut self, track_id: &str) -> bool {
        self.original_ids.retain(|id| id != track_id);
        let Some(cur) = self.index else {
            self.ids.retain(|i| i != track_id);
            return false;
        };
        let current_removed = self.ids.get(cur).is_some_and(|i| i == track_id);
        let before = self.ids[..cur.min(self.ids.len())]
            .iter()
            .filter(|i| *i == track_id)
            .count();
        self.ids.retain(|i| i != track_id);
        let new = cur - before;
        self.index = if self.ids.is_empty() || (current_removed && new >= self.ids.len()) {
            None
        } else {
            Some(new)
        };
        current_removed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(ids: &[&str], cur: &str) -> PlayQueue {
        let mut q = PlayQueue::default();
        q.replace(ids.iter().map(|s| s.to_string()).collect(), cur);
        q
    }

    // @test RF-03
    #[test]
    fn substituir_aponta_para_a_faixa_pedida() {
        let queue = q(&["a", "b", "c"], "b");
        assert_eq!(queue.index(), Some(1));
        assert_eq!(queue.current(), Some("b"));
        assert_eq!(queue.index_i64(), 1);
    }

    // @test RF-03
    #[test]
    fn faixa_fora_da_lista_entra_no_comeco() {
        let queue = q(&["a", "b"], "z");
        assert_eq!(queue.ids(), ["z", "a", "b"]);
        assert_eq!(queue.current(), Some("z"));
    }

    // @test RF-03
    #[test]
    fn fila_vazia_tem_indice_menos_um() {
        let queue = PlayQueue::default();
        assert_eq!(queue.index_i64(), -1);
        assert!(queue.current().is_none());
        assert!(queue.is_empty());
    }

    // @test RF-03
    #[test]
    fn proxima_e_anterior_respeitam_as_pontas() {
        let queue = q(&["a", "b", "c"], "a");
        assert_eq!(queue.next_after(0), Some(1));
        assert_eq!(queue.next_after(2), None);
        assert_eq!(queue.prev_before(0), None);
        assert_eq!(queue.prev_before(2), Some(1));
    }

    // @test RF-03
    #[test]
    fn selecionar_usa_a_fila_existente() {
        let mut queue = q(&["a", "b", "c"], "a");
        assert!(queue.select("c"));
        assert_eq!(queue.index(), Some(2));
        assert!(!queue.select("zzz"));
        assert_eq!(queue.index(), Some(2));
    }

    // @test RF-03
    #[test]
    fn remover_antes_da_atual_ajusta_o_indice() {
        let mut queue = q(&["a", "b", "c"], "c");
        assert!(!queue.remove("a"));
        assert_eq!(queue.ids(), ["b", "c"]);
        assert_eq!(queue.current(), Some("c"));
    }

    // @test RF-03
    #[test]
    fn remover_a_atual_aponta_para_a_seguinte_ou_nada() {
        let mut queue = q(&["a", "b", "c"], "b");
        assert!(queue.remove("b"));
        assert_eq!(queue.current(), Some("c"));
        let mut queue = q(&["a", "b"], "b");
        assert!(queue.remove("b"));
        assert_eq!(queue.index(), None);
        assert_eq!(queue.ids(), ["a"]);
        let mut queue = q(&["a"], "a");
        assert!(queue.remove("a"));
        assert!(queue.is_empty());
        assert_eq!(queue.index_i64(), -1);
    }

    // @test RF-03 RF-24
    #[test]
    fn remover_remove_todas_as_ocorrencias() {
        let mut queue = PlayQueue::default();
        queue.replace(vec!["a".into(), "x".into(), "b".into(), "x".into()], "b");
        assert!(!queue.remove("x"));
        assert_eq!(queue.ids(), ["a", "b"]);
        assert_eq!(queue.current(), Some("b"));
        assert_eq!(queue.original_ids, ["a", "b"]);
    }
}
