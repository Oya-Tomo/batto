PREFIX ?= /usr/local
BINDIR = $(PREFIX)/bin

.PHONY: build install setup uninstall service enable disable

build:
	cargo build --release

install:
	install -d $(DESTDIR)$(BINDIR)
	install -m 755 target/release/batto target/release/battod $(DESTDIR)$(BINDIR)

setup:
	install -d $(HOME)/.config/systemd/user
	sed 's|__BINDIR__|$(BINDIR)|g' contrib/battod.service.in > $(HOME)/.config/systemd/user/battod.service
	systemctl --user daemon-reload

enable:
	systemctl --user enable --now battod

disable:
	systemctl --user disable --now battod

uninstall:
	make disable
	rm -f $(HOME)/.config/systemd/user/battod.service
	systemctl --user daemon-reload
	sudo rm -f $(DESTDIR)$(BINDIR)/batto $(DESTDIR)$(BINDIR)/battod
