%define __spec_install_post %{nil}
%define __os_install_post %{_dbpath}/brp-compress
%define debug_package %{nil}

Name: ox
Summary: A Rust powered text editor.
Version: @@VERSION@@
Release: @@RELEASE@@%{?dist}
License: GPL-2.0
Group: Applications/System
Source0: %{name}-%{version}.tar.gz
URL: https://github.com/curlpipe/ox

BuildRoot: %{_tmppath}/%{name}-%{version}-%{release}-root

%description
%{summary}

%prep
%setup -q

%install
rm -rf %{buildroot}
mkdir -p %{buildroot}
cp -a * %{buildroot}

%clean
rm -rf %{buildroot}

%files
%defattr(-,root,root,-)
%{_bindir}/*
