package sweep

// The set operations sweep.py writes with `in` and `&`.

// intersects is `bool(names & set(others))`.
func intersects(names map[string]bool, others []string) bool {
	for _, o := range others {
		if names[o] {
			return true
		}
	}
	return false
}
