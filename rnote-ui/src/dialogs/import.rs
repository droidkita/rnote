use adw::prelude::*;
use gettextrs::gettext;
use gtk4::{
    gio, glib, glib::clone, Builder, Dialog, FileChooserAction, FileChooserNative, FileFilter,
    Label, ResponseType, SpinButton, ToggleButton,
};
use num_traits::ToPrimitive;
use rnote_engine::engine::import::{PdfImportPageSpacing, PdfImportPagesType};

use crate::canvas::RnoteCanvas;
use crate::{config, RnoteAppWindow};

/// Asks to open the given file as rnote file and overwrites the current document.
#[allow(unused)]
pub(crate) fn dialog_open_overwrite(
    appwindow: &RnoteAppWindow,
    canvas: &RnoteCanvas,
    input_file: gio::File,
) {
    let builder = Builder::from_resource(
        (String::from(config::APP_IDPATH) + "ui/dialogs/import.ui").as_str(),
    );
    let dialog: adw::MessageDialog = builder.object("dialog_open_overwrite").unwrap();

    dialog.set_transient_for(Some(appwindow));

    dialog.connect_response(
        None,
        clone!(@weak canvas, @weak appwindow => move |_dialog_open_input_file, response| {
            let input_file = input_file.clone();
            let open_overwrite = |appwindow: &RnoteAppWindow, canvas: &RnoteCanvas| {
                if let Err(e) = appwindow.load_in_file(input_file, None, canvas) {
                    log::error!("failed to load in input file, {e:?}");
                    appwindow.overlays().dispatch_toast_error(&gettext("Opening file failed."));
                }
            };

            match response {
                "discard" => {
                    open_overwrite(&appwindow, &canvas);
                }
                "save" => {
                    glib::MainContext::default().spawn_local(clone!(@weak canvas, @weak appwindow => async move {
                        if let Some(output_file) = canvas.output_file() {
                            appwindow.overlays().start_pulsing_progressbar();

                            if let Err(e) = canvas.save_document_to_file(&output_file).await {
                                canvas.set_output_file(None);

                                log::error!("saving document failed with error `{e:?}`");
                                appwindow.overlays().dispatch_toast_error(&gettext("Saving document failed."));
                            }

                            appwindow.overlays().finish_progressbar();
                            // No success toast on saving without dialog, success is already indicated in the header title
                        } else {
                            // Open a dialog to choose a save location
                            super::export::filechooser_save_doc_as(&appwindow, &canvas);
                        }

                        // only open and overwrite document if saving was successful
                        if !canvas.unsaved_changes() {
                            open_overwrite(&appwindow, &canvas);
                        }
                    }));
                },
                _ => {
                // Cancel
                }
            }
        }),
    );

    dialog.show();
}

/// Opens a new rnote save file in a new tab
pub(crate) fn filechooser_open_doc(appwindow: &RnoteAppWindow) {
    let filter = FileFilter::new();
    filter.add_mime_type("application/rnote");
    filter.add_suffix("rnote");
    filter.set_name(Some(&gettext(".rnote")));

    let filechooser: FileChooserNative = FileChooserNative::builder()
        .title(&gettext("Open file"))
        .modal(true)
        .transient_for(appwindow)
        .accept_label(&gettext("Open"))
        .cancel_label(&gettext("Cancel"))
        .action(FileChooserAction::Open)
        .select_multiple(false)
        .build();

    filechooser.set_filter(&filter);

    if let Some(current_workspace_dir) = appwindow.workspacebrowser().dirlist_dir() {
        if let Err(e) =
            filechooser.set_current_folder(Some(&gio::File::for_path(current_workspace_dir)))
        {
            log::error!("set_current_folder() for dialog_open_doc failed with Err: {e:?}");
        }
    }

    filechooser.connect_response(clone!(@weak appwindow => move |filechooser, responsetype| {
        match responsetype {
            ResponseType::Accept => {
                if let Some(input_file) = filechooser.file() {
                    appwindow.open_file_w_dialogs(input_file, None);
                }
            },
            _ => {}
        }

    }));

    filechooser.show();

    // keeping the filechooser around because otherwise GTK won't keep it alive
    *appwindow.filechoosernative().borrow_mut() = Some(filechooser);
}

pub(crate) fn filechooser_import_file(appwindow: &RnoteAppWindow) {
    let filter = FileFilter::new();
    filter.add_mime_type("application/x-xopp");
    filter.add_mime_type("application/pdf");
    filter.add_mime_type("image/svg+xml");
    filter.add_mime_type("image/png");
    filter.add_mime_type("image/jpeg");
    filter.add_suffix("xopp");
    filter.add_suffix("pdf");
    filter.add_suffix("svg");
    filter.add_suffix("png");
    filter.add_suffix("jpg");
    filter.add_suffix("jpeg");
    filter.set_name(Some(&gettext("JPG, PDF, PNG, SVG, Xopp")));

    let filechooser: FileChooserNative = FileChooserNative::builder()
        .title(&gettext("Import file"))
        .modal(true)
        .transient_for(appwindow)
        .accept_label(&gettext("Import"))
        .cancel_label(&gettext("Cancel"))
        .action(FileChooserAction::Open)
        .select_multiple(false)
        .build();

    filechooser.set_filter(&filter);

    if let Some(current_workspace_dir) = appwindow.workspacebrowser().dirlist_dir() {
        if let Err(e) =
            filechooser.set_current_folder(Some(&gio::File::for_path(current_workspace_dir)))
        {
            log::error!("set_current_folder() for dialog_import_file failed with Err: {e:?}");
        }
    }

    filechooser.connect_response(clone!(@weak appwindow => move |filechooser, responsetype| {
        match responsetype {
            ResponseType::Accept => {
                if let Some(input_file) = filechooser.file() {
                    appwindow.open_file_w_dialogs(input_file, None);
                }
            }
            _ => {
            }
        }
    }));

    filechooser.show();
    // keeping the filechooser around because otherwise GTK won't keep it alive
    *appwindow.filechoosernative().borrow_mut() = Some(filechooser);
}

pub(crate) fn dialog_import_pdf_w_prefs(
    appwindow: &RnoteAppWindow,
    canvas: &RnoteCanvas,
    input_file: gio::File,
    target_pos: Option<na::Vector2<f64>>,
) {
    let builder = Builder::from_resource(
        (String::from(config::APP_IDPATH) + "ui/dialogs/import.ui").as_str(),
    );
    let dialog: Dialog = builder.object("dialog_import_pdf_w_prefs").unwrap();
    let pdf_page_start_spinbutton: SpinButton =
        builder.object("pdf_page_start_spinbutton").unwrap();
    let pdf_page_end_spinbutton: SpinButton = builder.object("pdf_page_end_spinbutton").unwrap();
    let pdf_info_label: Label = builder.object("pdf_info_label").unwrap();
    let pdf_import_width_perc_spinbutton: SpinButton =
        builder.object("pdf_import_width_perc_spinbutton").unwrap();
    let pdf_import_page_spacing_row: adw::ComboRow =
        builder.object("pdf_import_page_spacing_row").unwrap();
    let pdf_import_as_bitmap_toggle: ToggleButton =
        builder.object("pdf_import_as_bitmap_toggle").unwrap();
    let pdf_import_as_vector_toggle: ToggleButton =
        builder.object("pdf_import_as_vector_toggle").unwrap();
    let pdf_import_bitmap_scalefactor_row: adw::ActionRow =
        builder.object("pdf_import_bitmap_scalefactor_row").unwrap();
    let pdf_import_bitmap_scalefactor_spinbutton: SpinButton = builder
        .object("pdf_import_bitmap_scalefactor_spinbutton")
        .unwrap();

    dialog.set_transient_for(Some(appwindow));

    let pdf_import_prefs = canvas.engine().borrow().import_prefs.pdf_import_prefs;

    // Set the widget state from the pdf import prefs
    pdf_page_start_spinbutton.set_increments(1.0, 2.0);
    pdf_page_end_spinbutton.set_increments(1.0, 2.0);
    pdf_import_width_perc_spinbutton.set_value(pdf_import_prefs.page_width_perc);
    match pdf_import_prefs.pages_type {
        PdfImportPagesType::Bitmap => {
            pdf_import_as_bitmap_toggle.set_active(true);
            pdf_import_bitmap_scalefactor_row.set_sensitive(true);
        }
        PdfImportPagesType::Vector => {
            pdf_import_as_vector_toggle.set_active(true);
            pdf_import_bitmap_scalefactor_row.set_sensitive(false);
        }
    }
    pdf_import_page_spacing_row.set_selected(pdf_import_prefs.page_spacing.to_u32().unwrap());
    pdf_import_bitmap_scalefactor_spinbutton.set_value(pdf_import_prefs.bitmap_scalefactor);

    pdf_page_start_spinbutton
        .bind_property("value", &pdf_page_end_spinbutton.adjustment(), "lower")
        .sync_create()
        .build();
    pdf_page_end_spinbutton
        .bind_property("value", &pdf_page_start_spinbutton.adjustment(), "upper")
        .sync_create()
        .build();

    // Update preferences
    pdf_import_as_vector_toggle.connect_toggled(
        clone!(@weak pdf_import_bitmap_scalefactor_row, @weak canvas, @weak appwindow => move |toggle| {
            if toggle.is_active() {
                canvas.engine().borrow_mut().import_prefs.pdf_import_prefs.pages_type = PdfImportPagesType::Vector;
                pdf_import_bitmap_scalefactor_row.set_sensitive(false);
            }
        }),
    );

    pdf_import_as_bitmap_toggle.connect_toggled(
        clone!(@weak pdf_import_bitmap_scalefactor_row, @weak canvas, @weak appwindow => move |toggle| {
            if toggle.is_active() {
                canvas.engine().borrow_mut().import_prefs.pdf_import_prefs.pages_type = PdfImportPagesType::Bitmap;
                pdf_import_bitmap_scalefactor_row.set_sensitive(true);
            }
        }),
    );

    pdf_import_bitmap_scalefactor_spinbutton.connect_value_changed(clone!(@weak canvas, @weak appwindow => move |spinbutton| {
        canvas.engine().borrow_mut().import_prefs.pdf_import_prefs.bitmap_scalefactor = spinbutton.value();
    }));

    pdf_import_page_spacing_row.connect_selected_notify(
        clone!(@weak canvas, @weak appwindow => move |row| {
            let page_spacing = PdfImportPageSpacing::try_from(row.selected()).unwrap();

            canvas.engine().borrow_mut().import_prefs.pdf_import_prefs.page_spacing = page_spacing;
        }),
    );

    pdf_import_width_perc_spinbutton.connect_value_changed(clone!(@weak canvas, @weak appwindow => move |spinbutton| {
        canvas.engine().borrow_mut().import_prefs.pdf_import_prefs.page_width_perc = spinbutton.value();
    }));

    if let Ok(poppler_doc) =
        poppler::Document::from_gfile(&input_file, None, None::<&gio::Cancellable>)
    {
        let file_name = input_file.basename().map_or_else(
            || gettext("- no file name -"),
            |s| s.to_string_lossy().to_string(),
        );
        let title = poppler_doc
            .title()
            .map_or_else(|| gettext("- no title -"), |s| s.to_string());
        let author = poppler_doc
            .author()
            .map_or_else(|| gettext("- no author -"), |s| s.to_string());
        let mod_date = poppler_doc
            .mod_datetime()
            .and_then(|dt| dt.format("%F").ok())
            .map_or_else(|| gettext("- no date -"), |s| s.to_string());
        let n_pages = poppler_doc.n_pages();

        // pdf info
        pdf_info_label.set_label(
            (String::from("")
                + "<b>"
                + &gettext("File name:")
                + "  </b>"
                + &format!("{file_name}\n")
                + "<b>"
                + &gettext("Title:")
                + "  </b>"
                + &format!("{title}\n")
                + "<b>"
                + &gettext("Author:")
                + "  </b>"
                + &format!("{author}\n")
                + "<b>"
                + &gettext("Modification date:")
                + "  </b>"
                + &format!("{mod_date}\n")
                + "<b>"
                + &gettext("Pages:")
                + "  </b>"
                + &format!("{n_pages}\n"))
                .as_str(),
        );

        // Configure pages spinners
        pdf_page_start_spinbutton.set_range(1.into(), n_pages.into());
        pdf_page_start_spinbutton.set_value(1.into());

        pdf_page_end_spinbutton.set_range(1.into(), n_pages.into());
        pdf_page_end_spinbutton.set_value(n_pages.into());
    }

    dialog.connect_response(
        clone!(@weak canvas, @weak appwindow => move |dialog, responsetype| {
            match responsetype {
                ResponseType::Apply => {
                    dialog.close();

                    let page_range = (pdf_page_start_spinbutton.value() as u32 - 1)..pdf_page_end_spinbutton.value() as u32;

                    glib::MainContext::default().spawn_local(clone!(@strong input_file, @weak canvas, @weak appwindow => async move {
                        appwindow.overlays().start_pulsing_progressbar();

                        let result = input_file.load_bytes_future().await;

                        if let Ok((file_bytes, _)) = result {
                            if let Err(e) = canvas.load_in_pdf_bytes(file_bytes.to_vec(), target_pos, Some(page_range)).await {
                                appwindow.overlays().dispatch_toast_error(&gettext("Opening PDF file failed."));
                                log::error!(
                                    "load_in_rnote_bytes() failed in dialog import pdf with Err: {e:?}"
                                );
                            }
                        }

                        appwindow.overlays().finish_progressbar();
                    }));
                }
                ResponseType::Cancel => {
                    dialog.close();
                }
                _ => {
                    dialog.close();
                }
            }
        }),
    );

    dialog.show();
}

pub(crate) fn dialog_import_xopp_w_prefs(
    appwindow: &RnoteAppWindow,
    canvas: &RnoteCanvas,
    input_file: gio::File,
) {
    let builder = Builder::from_resource(
        (String::from(config::APP_IDPATH) + "ui/dialogs/import.ui").as_str(),
    );
    let dialog: Dialog = builder.object("dialog_import_xopp_w_prefs").unwrap();
    let dpi_spinbutton: SpinButton = builder.object("xopp_import_dpi_spinbutton").unwrap();

    dialog.set_transient_for(Some(appwindow));

    let xopp_import_prefs = canvas.engine().borrow().import_prefs.xopp_import_prefs;

    // Set initial widget state for preference
    dpi_spinbutton.set_value(xopp_import_prefs.dpi);

    // Update preferences
    dpi_spinbutton.connect_changed(clone!(@weak canvas, @weak appwindow => move |spinbutton| {
        canvas.engine().borrow_mut().import_prefs.xopp_import_prefs.dpi = spinbutton.value();
    }));

    dialog.connect_response(
        clone!(@weak canvas, @weak appwindow => move |dialog, responsetype| {
            match responsetype {
                ResponseType::Apply => {
                    dialog.close();

                    glib::MainContext::default().spawn_local(clone!(@strong input_file, @weak appwindow => async move {
                        appwindow.overlays().start_pulsing_progressbar();

                        let result = input_file.load_bytes_future().await;

                        if let Ok((file_bytes, _)) = result {
                            if let Err(e) = canvas.load_in_xopp_bytes(file_bytes.to_vec()).await {
                                appwindow.overlays().dispatch_toast_error(&gettext("Opening Xournal++ file failed."));
                                log::error!(
                                    "load_in_xopp_bytes() failed in dialog import xopp with Err: {e:?}"
                                );
                            }
                        }

                        appwindow.overlays().finish_progressbar();
                    }));
                }
                ResponseType::Cancel => {
                    dialog.close();
                }
                _ => {
                    dialog.close();
                }
            }
        }),
    );

    dialog.show();
}
