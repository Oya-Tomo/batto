PREFIX ?= /usr/local
BINDIR = $(PREFIX)/bin
UNITDIR = $(HOME)/.config/systemd/user

.PHONY: build install uninstall service enable disable

build:
	cargo build --release

install: build service
	install -d $(DESTDIR)$(BINDIR)
	install -m 755 target/release/batto target/release/battod $(DESTDIR)$(BINDIR)
	install -d $(UNITDIR)
	sed 's|__BINDIR__|$(BINDIR)|g' contrib/battod.service.in > $(UNITDIR)/battod.service
	systemctl --user daemon-reload

service:
	@mkdir -p $(UNITDIR)
	@sed 's|__BINDIR__|$(BINDIR)|g' contrib/battod.service.in > $(UNITDIR)/battod.service
	@systemctl --user daemon-reload

enable:
	systemctl --user enable --now battod

disable:
	systemctl --user disable --now battod

uninstall:
	systemctl --user disable --now battod 2>/dev/null || true
	rm -f $(DESTDIR)$(BINDIR)/batto $(DESTDIR)$(BINDIR)/battod
	rm -f $(UNITDIR)/battod.service
	systemctl --user daemon-reload
