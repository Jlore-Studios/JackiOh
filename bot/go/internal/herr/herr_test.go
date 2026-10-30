package herr

import "testing"

func TestCatchAndTry(t *testing.T) {
	gh, ok := Catch[*GitHubError](func() { Raise(&GitHubError{Msg: "gone", Status: 404}) })
	if !ok || gh.Status != 404 {
		t.Fatal("Catch should take the GitHubError")
	}
	err := Try(func() {
		Catch[*GitHubError](func() { Raise(&GitError{Msg: "conflict"}) })
		t.Fatal("a GitError should pass a Catch for GitHubError")
	})
	if err == nil || err.Error() != "conflict" || !IsHarness(err) {
		t.Fatalf("Try should take what Catch passed on: %v", err)
	}
	if _, ok := CatchHarness(func() { Raise(&StateConflict{Msg: "x"}) }); !ok {
		t.Fatal("CatchHarness should take every subclass")
	}
	var m map[string]int
	if err := Try(func() { m["x"] = 1 }); err == nil || IsHarness(err) {
		t.Fatal("Try should take a runtime error as except Exception does")
	}
	if err := Try(func() { panic("plain") }); err == nil || err.Error() != "plain" {
		t.Fatal("Try should take a non-error panic")
	}
}
