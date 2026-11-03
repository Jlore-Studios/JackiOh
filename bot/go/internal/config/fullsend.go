package config

// Fullsend (#505, #2748 section 2).
//
// `fullsend_adopt`: a queued parent (`bot:build`, by a label, an assignment, `/harness build` or an
// issue form) that has open sub-issues is adopted as a fullsend tree by itself (`queue.AutoAdopt`),
// as `/harness fullsend` adopts one, with a comment that says so. A config that leaves it out (or
// sets it false, as Squishy's does) builds such a parent as it is asked.
//
// `fullsend_send_backs`: how many times the reconcile may send one part back to the subscription
// that built it (`deliver.SendBack`), each with its findings; past it the reconcile fixes the part
// itself. A config that leaves it out, or sets it to 0 (Squishy's), sends none back.
func init() {
	optional.Set("fullsend_adopt", false)
	optional.Set("fullsend_send_backs", 0)
}

// SendsBack is: this bot's reconcile may send a part back at all (`fullsend_send_backs` above 0).
func (x *Config) SendsBack() bool { return x.FullsendSendBacks > 0 }
