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

    // @req RF-49
    pub fn enqueue(&mut self, ids: Vec<String>, next: bool) {
        let at = if next {
            self.index.map_or(self.ids.len(), |i| i + 1)
        } else {
            self.ids.len()
        };
        let original_at = if next {
            self.index
                .map_or(self.original_ids.len(), |i| self.original_position(i) + 1)
        } else {
            self.original_ids.len()
        };
        self.original_ids
            .splice(original_at..original_at, ids.iter().cloned());
        self.ids.splice(at..at, ids);
    }

    // Identifica uma ocorrência repetida pela posição ordinal desse ID na fila visível.
    fn original_position(&self, index: usize) -> usize {
        let id = &self.ids[index];
        let ordinal = self.ids[..=index].iter().filter(|item| *item == id).count();
        self.original_ids
            .iter()
            .enumerate()
            .filter(|(_, item)| *item == id)
            .nth(ordinal - 1)
            .map_or(0, |(i, _)| i)
    }

    // @req RF-49
    pub fn remove_at(&mut self, index: usize) -> bool {
        if index >= self.ids.len() || self.index == Some(index) {
            return false;
        }
        let original = self.original_position(index);
        self.ids.remove(index);
        self.original_ids.remove(original);
        if let Some(current) = self.index
            && index < current
        {
            self.index = Some(current - 1);
        }
        true
    }

    // @req RF-49
    pub fn move_at(&mut self, from: usize, to: usize, shuffle_off: bool) -> bool {
        if from >= self.ids.len() || to >= self.ids.len() {
            return false;
        }
        if from == to {
            return true;
        }
        let id = self.ids.remove(from);
        self.ids.insert(to, id);
        if shuffle_off {
            // Em modo off, a ordem original passa a ser a de reprodução: depois de um ciclo de
            // aleatório, as duas podem ter divergido no prefixo já tocado.
            self.original_ids = self.ids.clone();
        }
        if let Some(current) = self.index {
            self.index = Some(if current == from {
                to
            } else if from < current && to >= current {
                current - 1
            } else if from > current && to <= current {
                current + 1
            } else {
                current
            });
        }
        true
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

    // @test RF-49
    #[test]
    fn enfileirar_no_fim_e_depois_da_atual_preserva_a_atual() {
        let mut queue = q(&["a", "b", "c"], "b");
        queue.enqueue(vec!["d".into(), "e".into()], false);
        queue.enqueue(vec!["x".into(), "y".into()], true);
        assert_eq!(queue.ids(), ["a", "b", "x", "y", "c", "d", "e"]);
        assert_eq!(queue.original_ids, queue.ids);
        assert_eq!(queue.current(), Some("b"));
    }

    // @test RF-49
    #[test]
    fn enfileirar_em_fila_vazia_nao_seleciona_faixa() {
        let mut queue = PlayQueue::default();
        queue.enqueue(vec!["a".into(), "b".into()], true);
        assert_eq!(queue.ids(), ["a", "b"]);
        assert_eq!(queue.index_i64(), -1);
    }

    // @test RF-49
    #[test]
    fn remover_ocorrencia_antes_e_depois_da_atual() {
        let mut queue = q(&["a", "x", "b", "x", "c"], "b");
        assert!(queue.remove_at(1));
        assert_eq!(queue.index(), Some(1));
        assert!(queue.remove_at(2));
        assert_eq!(queue.ids(), ["a", "b", "c"]);
        assert_eq!(queue.original_ids, ["a", "b", "c"]);
    }

    // @test RF-49
    #[test]
    fn remover_atual_ou_indice_invalido_nao_altera_fila() {
        let mut queue = q(&["a", "b"], "b");
        let before = queue.clone();
        assert!(!queue.remove_at(1));
        assert!(!queue.remove_at(2));
        assert_eq!(queue, before);
    }

    // @test RF-49
    #[test]
    fn mover_atual_e_cruzar_sua_posicao() {
        let mut queue = q(&["a", "b", "c", "d"], "b");
        assert!(queue.move_at(1, 3, true));
        assert_eq!(queue.ids(), ["a", "c", "d", "b"]);
        assert_eq!(queue.index(), Some(3));
        assert!(queue.move_at(0, 3, true));
        assert_eq!(queue.index(), Some(2));
        assert!(queue.move_at(3, 0, true));
        assert_eq!(queue.index(), Some(3));
        assert_eq!(queue.original_ids, queue.ids);
        assert!(!queue.move_at(8, 0, true));
    }

    // @test RF-49 RF-24
    #[test]
    fn enfileirar_embaralhar_remover_e_restaurar_com_repetidos() {
        let mut queue = q(&["a", "b", "c"], "a");
        queue.enqueue(vec!["b".into(), "d".into()], true);
        queue.set_ids(
            vec!["a", "c", "d", "b", "b"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            0,
        );
        assert!(queue.remove_at(4));
        assert!(queue.move_at(2, 1, false));
        let restored = queue.original_order_of(&queue.ids()[..1], &queue.ids()[1..]);
        assert_eq!(restored, ["b", "d", "c"]);
    }

    // @test RF-49 RF-24
    #[test]
    fn mover_em_off_depois_do_aleatorio_segue_a_ordem_visivel() {
        let mut queue = PlayQueue::default();
        let ids = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        queue.replace(ids(&["a", "b", "c", "d", "e"]), "a");
        // Depois de um ciclo de aleatório e da volta para off, com d tocando.
        queue.set_ids(ids(&["a", "d", "b", "c", "e"]), 1);
        assert!(queue.move_at(4, 2, true));
        assert_eq!(queue.ids, ids(&["a", "d", "e", "b", "c"]));
        assert_eq!(queue.original_ids, queue.ids);
        assert_eq!(queue.index(), Some(1));
    }
}
